use std::convert::Infallible;

use axum::extract::{Path, State};
use axum::response::sse::Event;
use axum::response::Sse;
use axum::Json;
use futures::stream::{self, Stream};
use futures::StreamExt;
use uuid::Uuid;

use crate::modules::auth::models::UserContext;
use crate::modules::query::models::{QueryRequest, StreamEvent};
use crate::modules::query::service::{JobStatus, QueryService};
use crate::shared::error::AppError;

fn sse_json_event(stream_event: StreamEvent) -> Event {
    let data_json = serde_json::to_string(&stream_event).unwrap_or_else(|_| {
        r#"{"type":"error","data":{"text":"serialization failed"}}"#.to_string()
    });
    Event::default().data(data_json)
}

/// POST `/api/query`
///
/// Accepts a `QueryRequest` JSON body and returns a Server-Sent Events (SSE)
/// stream with the RAG pipeline result.
///
/// Event types yielded:
/// - `data: {"type":"chunk","text":"..."}`
/// - `data: {"type":"sources","sources":[...]}`
/// - `data: {"type":"error","text":"..."}`
/// - `data: {"type":"done"}`
pub async fn query_handler(
    user_ctx: UserContext,
    State(svc): State<QueryService>,
    Json(request): Json<QueryRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    tracing::info!(
        component = "query/handlers",
        collection_id = %request.collection_id,
        query_length = request.query.len(),
        user_id = %user_ctx.user_id,
        "query.handler.invoked"
    );

    let event_stream = svc
        .process_query(request, &user_ctx.user_id, user_ctx.is_admin())
        .await?;

    // Map internal StreamEvents into SSE Event objects
    let sse_stream = event_stream.map(|result| {
        result.map(|stream_event| {
            // Serialize the whole event into a data JSON string
            let data_json = serde_json::to_string(&stream_event).unwrap_or_else(|_| {
                r#"{"type":"error","text":"serialization failed"}"#.to_string()
            });

            Event::default().data(data_json)
        })
    });

    Ok(Sse::new(sse_stream))
}

/// GET `/api/query/:session_id/subscribe`
///
/// Recovery endpoint used after a browser reload while a RAG pipeline is still
/// running. It emits exactly one SSE event:
/// - `data: {"type":"done","data":{}}` when the active pipeline finishes
/// - `data: {"type":"error","data":{"text":"..."}}` when it fails
///
/// If the in-memory job is already gone but the assistant message is persisted,
/// the endpoint emits `done` immediately. Otherwise it returns 404 so the client
/// can stop recovery instead of polling.
pub async fn subscribe_handler(
    user_ctx: UserContext,
    State(svc): State<QueryService>,
    Path(session_id): Path<Uuid>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    tracing::info!(
        component = "query/handlers",
        session_id = %session_id,
        user_id = %user_ctx.user_id,
        "query.subscribe.invoked"
    );

    // Enforce the same session visibility rules as the session endpoints.
    svc.conversation_repo
        .get_session_for_user(session_id, &user_ctx.user_id, user_ctx.is_admin())
        .await?;

    let receiver = svc.active_jobs.get_receiver(session_id);
    let has_completed_assistant = if receiver.is_none() {
        let messages = svc.conversation_repo.get_messages(session_id).await?;
        messages
            .iter()
            .any(|msg| msg.role == "assistant" && !msg.content.trim().is_empty())
    } else {
        false
    };

    if receiver.is_none() && !has_completed_assistant {
        return Err(AppError::NotFound(format!(
            "No active pipeline for session {session_id}"
        )));
    }

    let sse_stream = stream::once(async move {
        if let Some(mut rx) = receiver {
            match rx.borrow().clone() {
                JobStatus::Done => {
                    return Ok(sse_json_event(StreamEvent {
                        event_type: "done".to_string(),
                        data: serde_json::json!({}),
                    }));
                }
                JobStatus::Failed(error) => {
                    return Ok(sse_json_event(StreamEvent {
                        event_type: "error".to_string(),
                        data: serde_json::json!({"text": error}),
                    }));
                }
                JobStatus::Running => {}
            }

            if rx.changed().await.is_err() {
                return Ok(sse_json_event(StreamEvent {
                    event_type: "error".to_string(),
                    data: serde_json::json!({"text": "Pipeline recovery stream closed"}),
                }));
            }

            match rx.borrow().clone() {
                JobStatus::Done => Ok(sse_json_event(StreamEvent {
                    event_type: "done".to_string(),
                    data: serde_json::json!({}),
                })),
                JobStatus::Failed(error) => Ok(sse_json_event(StreamEvent {
                    event_type: "error".to_string(),
                    data: serde_json::json!({"text": error}),
                })),
                JobStatus::Running => Ok(sse_json_event(StreamEvent {
                    event_type: "error".to_string(),
                    data: serde_json::json!({"text": "Pipeline recovery ended before completion"}),
                })),
            }
        } else {
            Ok(sse_json_event(StreamEvent {
                event_type: "done".to_string(),
                data: serde_json::json!({}),
            }))
        }
    });

    Ok(Sse::new(sse_stream))
}
