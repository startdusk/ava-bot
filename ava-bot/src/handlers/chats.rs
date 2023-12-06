use std::{convert::Infallible, time::Duration};

use axum::response::{
    sse::{Event, KeepAlive},
    IntoResponse, Sse,
};
use futures::stream;
use tokio_stream::StreamExt as _;
use tracing::info;

pub async fn chats_handlers() -> impl IntoResponse {
    info!("user connected");

    let stream = stream::repeat_with(|| Event::default().data("hi"))
        .map(Ok::<_, Infallible>)
        .throttle(Duration::from_secs(1));

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(1))
            .text("keep-alive-text"),
    )
}
