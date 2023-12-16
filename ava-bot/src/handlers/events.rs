use std::{convert::Infallible, sync::Arc, time::Duration};

use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive},
        IntoResponse, Sse,
    },
};
use dashmap::DashMap;
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, StreamExt as _};
use tracing::info;

use crate::{extractors::AppContext, AppState};

use super::AssistantEvent;

const MAX_EVENTS: usize = 128;

pub async fn events_handler(
    context: AppContext,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    info!("user connected for chats");
    sse_handler(context, &state.events).await
}

pub async fn sse_handler(
    context: AppContext,
    map: &DashMap<String, broadcast::Sender<AssistantEvent>>,
) -> impl IntoResponse {
    let device_id = &context.device_id;
    let rx = if let Some(tx) = map.get(device_id) {
        tx.subscribe()
    } else {
        let (tx, rx) = broadcast::channel(MAX_EVENTS);
        map.insert(device_id.to_string(), tx);
        rx
    };

    // wrap receiver in a stream
    let stream = BroadcastStream::new(rx)
        .filter_map(|v| v.ok())
        .map(|v| {
            let (event, id) = match &v {
                AssistantEvent::Signal(_) => ("signal", "".to_string()),
                AssistantEvent::InputSkeleton(_) => ("input_skeleton", "".to_string()),
                AssistantEvent::Input(v) => ("input", v.id.to_string()),
                AssistantEvent::ReplySkeleton(_) => ("reply_skeleton", "".to_string()),
                AssistantEvent::Reply(v) => ("reply", v.id.to_string()),
            };
            let data: String = v.into();
            Event::default().data(data).event(event).id(id)
        })
        .map(Ok::<_, Infallible>);

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive-text"),
    )
}
