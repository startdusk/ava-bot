use askama::Template;
use std::sync::Arc;
use tokio::{fs, sync::broadcast};

use anyhow::anyhow;
use axum::{
    extract::{Multipart, State},
    response::IntoResponse,
    Json,
};
use llm_sdk::{
    chat_completion::{ChatCompletionMessage, ChatCompletionRequest},
    speech::SpeechRequest,
    whisper::{WhisperRequestBuilder, WhisperRequestType},
    LlmSdk,
};
use serde_json::json;
use uuid::Uuid;

use crate::{audio_path, audio_url, error::AppError, extractors::AppContext, AppState};

use super::{AssistantEvent, AssistantStep, ChatInputEvent, SpeechResult};

pub async fn assistant_handler(
    context: AppContext,
    State(state): State<Arc<AppState>>,
    data: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let device_id = &context.device_id;
    let signal_sender = state
        .signals
        .get(device_id)
        .ok_or_else(|| anyhow!("device_id not found for signal sender"))?
        .clone();
    let chat_sender = state
        .chats
        .get(device_id)
        .ok_or_else(|| anyhow!("device_id not found for chat sender"))?
        .clone();
    let llm = &state.llm;
    match process(&signal_sender, &chat_sender, device_id, llm, data).await {
        Ok(_) => Ok(Json(json!({"status": "done"}))),
        Err(e) => {
            signal_sender.send(error(e.to_string()))?;
            Ok(Json(json!({"status": "error"})))
        }
    }
}

async fn transcript(llm: &LlmSdk, data: &[u8]) -> anyhow::Result<String> {
    let req = WhisperRequestBuilder::default()
        .file(data.into())
        .prompt("If audio language is Chinese, please use Simplified Chinese")
        .request_type(WhisperRequestType::Transcription)
        .build()
        .unwrap();
    let res = llm.whisper(req).await?;
    Ok(res.text)
}

async fn chat_completion(llm: &LlmSdk, prompt: &str) -> anyhow::Result<String> {
    let req = ChatCompletionRequest::new(vec![
        ChatCompletionMessage::new_system(
            "I'm an assistant who can answer anything for you",
            "Ava",
        ),
        ChatCompletionMessage::new_user(prompt, ""),
    ]);

    let mut res = llm.chat_completion(req).await?;

    let content = res
        .choices
        .pop()
        .ok_or_else(|| anyhow!("expect at least the choice"))?
        .message
        .content
        .ok_or_else(|| anyhow!("expect content but no content available"))?;

    Ok(content)
}

async fn speech(llm: &LlmSdk, device_id: &str, text: &str) -> anyhow::Result<SpeechResult> {
    let req = SpeechRequest::new(text);
    let data = llm.speech(req).await?;
    let uuid = Uuid::new_v4().to_string();
    let path = audio_path(device_id, &uuid);
    if let Some(parent) = path.parent() {
        // 父级路径没有创建就创建它
        if !parent.exists() {
            fs::create_dir_all(parent).await?
        }
    }
    fs::write(&path, data).await?;
    Ok(SpeechResult::new(text, audio_url(device_id, &uuid)))
}

async fn process(
    signal_sender: &broadcast::Sender<String>,
    chat_sender: &broadcast::Sender<String>,
    device_id: &str,
    llm: &LlmSdk,
    mut data: Multipart,
) -> anyhow::Result<()> {
    signal_sender.send(in_audio_upload()).unwrap();
    let Some(field) = data.next_field().await? else {
        return Err(anyhow!("expected an audio field"))?;
    };

    let data = match field.name() {
        Some(name) if name == "audio" => field.bytes().await?,
        _ => return Err(anyhow!("expected an audio field"))?,
    };

    signal_sender.send(in_transcrition())?;

    let text = transcript(llm, &data).await?;
    chat_sender.send(ChatInputEvent::new(&text).into())?;

    signal_sender.send(in_chat_completion())?;
    let output = chat_completion(llm, &text).await?;
    signal_sender.send(in_speech())?;
    let speech_result = speech(llm, &device_id, &output).await?;
    signal_sender.send(complete())?;
    chat_sender.send(speech_result.into())?;

    Ok(())
}

fn in_audio_upload() -> String {
    AssistantEvent::Processing(AssistantStep::UploadAudio).into()
}

fn in_transcrition() -> String {
    AssistantEvent::Processing(AssistantStep::Transcrition).into()
}

fn in_chat_completion() -> String {
    AssistantEvent::Processing(AssistantStep::ChatCompletion).into()
}
fn in_speech() -> String {
    AssistantEvent::Processing(AssistantStep::Speech).into()
}

#[allow(dead_code)]
fn finsh_audio_upload() -> String {
    AssistantEvent::Finish(AssistantStep::UploadAudio).into()
}

#[allow(dead_code)]
fn finsh_transcrition() -> String {
    AssistantEvent::Finish(AssistantStep::Transcrition).into()
}

#[allow(dead_code)]
fn finsh_chat_completion() -> String {
    AssistantEvent::Finish(AssistantStep::ChatCompletion).into()
}

#[allow(dead_code)]
fn finsh_speech() -> String {
    AssistantEvent::Finish(AssistantStep::Speech).into()
}

fn error(msg: impl Into<String>) -> String {
    AssistantEvent::Error(msg.into()).into()
}

fn complete() -> String {
    AssistantEvent::Complete.render().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_render() {
        let event = error("error");
        assert_eq!(event, "\n<p class=\"text-red-800\">Error: error</p>\n")
    }
}
