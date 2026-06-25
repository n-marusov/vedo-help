use std::convert::Infallible;

use axum::extract::State;
use axum::response::sse::Event;
use axum::response::Sse;
use axum::Json;
use futures::stream::Stream;
use futures::StreamExt;

use crate::modules::query::models::QueryRequest;
use crate::modules::query::service::QueryService;
use crate::shared::error::AppError;

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
    State(svc): State<QueryService>,
    Json(request): Json<QueryRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    tracing::info!(
        component = "query/handlers",
        collection_id = %request.collection_id,
        query_length = request.query.len(),
        "query.handler.invoked"
    );

    let event_stream = svc.process_query(request).await?;

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
