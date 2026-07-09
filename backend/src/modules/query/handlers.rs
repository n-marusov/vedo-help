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

fn job_status_event(status: JobStatus) -> Option<StreamEvent> {
    match status {
        JobStatus::Running { stage: Some(stage) } => Some(StreamEvent {
            event_type: "pipeline_stage".to_string(),
            data: serde_json::json!({"stage_name": stage}),
        }),
        JobStatus::Running { stage: None } => None,
        JobStatus::Done => Some(StreamEvent {
            event_type: "done".to_string(),
            data: serde_json::json!({}),
        }),
        JobStatus::Failed(error) => Some(StreamEvent {
            event_type: "error".to_string(),
            data: serde_json::json!({"text": error}),
        }),
    }
}

fn is_terminal_job_status(status: &JobStatus) -> bool {
    matches!(status, JobStatus::Done | JobStatus::Failed(_))
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
/// running. It replays the currently known pipeline stage, then emits further
/// `pipeline_stage` updates until the active pipeline finishes with `done` or
/// `error`. If the in-memory job is already gone but the assistant message is
/// persisted, the endpoint emits `done` immediately. Otherwise it returns 404 so
/// the client can stop recovery instead of polling.
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

    let sse_stream = if let Some(rx) = receiver {
        stream::unfold((rx, false), |(mut rx, mut current_emitted)| async move {
            loop {
                let status = rx.borrow().clone();
                if current_emitted && is_terminal_job_status(&status) {
                    return None;
                }
                if !current_emitted {
                    current_emitted = true;
                    if let Some(event) = job_status_event(status.clone()) {
                        tracing::info!(
                            component = "query/handlers",
                            event_type = event.event_type.as_str(),
                            "[FIX] query.subscribe_recovery_event"
                        );
                        return Some((Ok(sse_json_event(event)), (rx, current_emitted)));
                    }
                    if is_terminal_job_status(&status) {
                        return None;
                    }
                }

                if rx.changed().await.is_err() {
                    let event = StreamEvent {
                        event_type: "error".to_string(),
                        data: serde_json::json!({"text": "Pipeline recovery stream closed"}),
                    };
                    return Some((Ok(sse_json_event(event)), (rx, true)));
                }
                current_emitted = false;
            }
        })
        .left_stream()
    } else {
        stream::once(async {
            Ok(sse_json_event(StreamEvent {
                event_type: "done".to_string(),
                data: serde_json::json!({}),
            }))
        })
        .right_stream()
    };

    Ok(Sse::new(sse_stream))
}
