mod assistant;
mod chats;
mod common;

use askama::Template;
pub use assistant::*;
pub use chats::*;
use chrono::Local;
pub use common::*;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Clone, Template, Serialize, Deserialize)]
#[template(path = "events/signal.html.j2")]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum AssistantEvent {
    Processing(AssistantStep),
    Finish(AssistantStep),
    Error(String),
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumString, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
enum AssistantStep {
    UploadAudio,
    Transcrition,
    ChatCompletion,
    Speech,
}

impl From<AssistantEvent> for String {
    fn from(event: AssistantEvent) -> Self {
        event.render().unwrap()
    }
}

#[derive(Debug, Clone, Template, Serialize, Deserialize)]
#[template(path = "events/chat_input.html.j2")]
struct ChatInputEvent {
    message: String,
    datetime: String,
    avatar: String,
    name: String,
}

impl ChatInputEvent {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            datetime: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            avatar: "https://i.pravatar.cc/300".to_string(),
            name: "User".to_string(),
        }
    }
}

impl From<ChatInputEvent> for String {
    fn from(event: ChatInputEvent) -> Self {
        event.render().unwrap()
    }
}

#[derive(Debug, Clone, Template, Serialize, Deserialize)]
#[template(path = "events/chat_reply.html.j2")]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum ChatReplyEvent {
    Speech(SpeechResult),
    // Image(ImageResult),
    // Markdown(MarkdownResult),
}

#[derive(Debug, Clone, Template, Serialize, Deserialize)]
#[template(path = "blocks/speech.html.j2")]
struct SpeechResult {
    text: String,
    url: String,
}

impl SpeechResult {
    pub fn new(text: impl Into<String>, url: impl Into<String>) -> Self {
        SpeechResult {
            text: text.into(),
            url: url.into(),
        }
    }
}

impl From<SpeechResult> for ChatReplyEvent {
    fn from(reuslt: SpeechResult) -> Self {
        ChatReplyEvent::Speech(reuslt)
    }
}

impl From<SpeechResult> for String {
    fn from(result: SpeechResult) -> Self {
        ChatReplyEvent::Speech(result).render().unwrap()
    }
}
