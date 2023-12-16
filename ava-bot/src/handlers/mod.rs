mod assistant;
mod common;
mod events;

use askama::Template;
pub use assistant::*;
use chrono::Local;
pub use common::*;
use derive_more::From;
pub use events::*;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

use crate::tools::{DrawImageResult, WriteCodeResult};

#[derive(Debug, Clone, From)]
pub(crate) enum AssistantEvent {
    Signal(SignalEvent),
    InputSkeleton(ChatInputSkeletonEvent),
    Input(ChatInputEvent),
    ReplySkeleton(ChatReplySkeletonEvent),
    Reply(ChatReplyEvent),
}

#[derive(Debug, Clone, Template, Serialize, Deserialize)]
#[template(path = "events/signal.html.j2")]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub(crate) enum SignalEvent {
    Processing(AssistantStep),
    Finish(AssistantStep),
    Error(String),
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumString, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub(crate) enum AssistantStep {
    UploadAudio,
    Transcrition,
    ChatCompletion,
    Thinking,
    DrawImage,
    WriteCode,
    Speech,
}

impl From<SignalEvent> for String {
    fn from(event: SignalEvent) -> Self {
        event.render().unwrap()
    }
}

#[derive(Debug, Clone, Template, Serialize, Deserialize)]
#[template(path = "events/chat_input_skeleton.html.j2")]
pub(crate) struct ChatInputSkeletonEvent {
    id: String,
    datetime: String,
    avatar: String,
    name: String,
}

#[derive(Debug, Clone, Template, Serialize, Deserialize)]
#[template(path = "events/chat_input.html.j2")]
pub(crate) struct ChatInputEvent {
    id: String,
    content: String,
}

#[derive(Debug, Clone, Template, Serialize, Deserialize)]
#[template(path = "events/chat_reply_skeleton.html.j2")]
pub(crate) struct ChatReplySkeletonEvent {
    id: String,
    avatar: String, // /public/images/ava-small.png
    name: String,   // Ava
}

#[derive(Debug, Clone, Template, Serialize, Deserialize)]
#[template(path = "events/chat_reply.html.j2")]
pub(crate) struct ChatReplyEvent {
    id: String,
    data: ChatReplyData,
}

impl ChatInputSkeletonEvent {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            datetime: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            avatar: "https://i.pravatar.cc/300".to_string(),
            name: "User".to_string(),
        }
    }
}

impl ChatInputEvent {
    pub fn new(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum ChatReplyData {
    Speech(SpeechResult),
    Image(DrawImageResult),
    Markdown(WriteCodeResult),
}

#[derive(Debug, Clone, Template, Serialize, Deserialize)]
#[template(path = "blocks/speech.html.j2")]
pub(crate) struct SpeechResult {
    text: String,
    url: String,
}

impl SpeechResult {
    pub(crate) fn new(text: impl Into<String>, url: impl Into<String>) -> Self {
        SpeechResult {
            text: text.into(),
            url: url.into(),
        }
    }

    pub(crate) fn new_text_only(text: impl Into<String>) -> Self {
        Self::new(text, "".to_string())
    }
}

impl ChatReplySkeletonEvent {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            avatar: "/public/images/ava-small.png".to_string(),
            name: "Ava".to_string(),
        }
    }
}

impl ChatReplyEvent {
    pub fn new(id: impl Into<String>, data: impl Into<ChatReplyData>) -> Self {
        Self {
            id: id.into(),
            data: data.into(),
        }
    }
}

impl From<SpeechResult> for String {
    fn from(result: SpeechResult) -> Self {
        result.render().unwrap()
    }
}

impl From<ChatInputEvent> for String {
    fn from(event: ChatInputEvent) -> Self {
        event.render().unwrap()
    }
}

impl From<ChatInputSkeletonEvent> for String {
    fn from(event: ChatInputSkeletonEvent) -> Self {
        event.render().unwrap()
    }
}

impl From<ChatReplyEvent> for String {
    fn from(event: ChatReplyEvent) -> Self {
        event.render().unwrap()
    }
}

impl From<ChatReplySkeletonEvent> for String {
    fn from(event: ChatReplySkeletonEvent) -> Self {
        event.render().unwrap()
    }
}

impl From<AssistantEvent> for String {
    fn from(event: AssistantEvent) -> Self {
        match event {
            AssistantEvent::Signal(v) => v.into(),
            AssistantEvent::InputSkeleton(v) => v.into(),
            AssistantEvent::Input(v) => v.into(),
            AssistantEvent::ReplySkeleton(v) => v.into(),
            AssistantEvent::Reply(v) => v.into(),
        }
    }
}
