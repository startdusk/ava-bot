use comrak::markdown_to_html;
use std::{str::FromStr, sync::Arc};
use tokio::{fs, sync::broadcast};

use anyhow::{anyhow, bail};
use axum::{
    extract::{Multipart, State},
    response::IntoResponse,
    Json,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use llm_sdk::{
    chat_completion::{ChatCompletionChoice, ChatCompletionMessage, ChatCompletionRequest},
    create_image::{CreateImageRequestBuilder, ImageResponseFormat},
    speech::SpeechRequest,
    whisper::{WhisperRequestBuilder, WhisperRequestType},
    LlmSdk,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    audio_path, audio_url,
    error::AppError,
    extractors::AppContext,
    image_path, image_url,
    tools::{
        tool_completion_request, AnswerArgs, AssistantTool, DrawImageArgs, DrawImageResult,
        WriteCodeArgs, WriteCodeResult,
    },
    AppState,
};

use super::{
    AssistantEvent, AssistantStep, ChatInputEvent, ChatInputSkeletonEvent, ChatReplyEvent,
    ChatReplySkeletonEvent, SignalEvent, SpeechResult,
};

pub async fn assistant_handler(
    context: AppContext,
    State(state): State<Arc<AppState>>,
    data: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let device_id = &context.device_id;
    let event_sender = state
        .events
        .get(device_id)
        .ok_or_else(|| anyhow!("device_id not found for signal sender"))?
        .clone();
    let llm = &state.llm;
    match process(&event_sender, device_id, llm, data).await {
        Ok(_) => Ok(Json(json!({"status": "done"}))),
        Err(e) => {
            event_sender.send(error(e.to_string()))?;
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

async fn chat_completion_with_tools(
    llm: &LlmSdk,
    prompt: &str,
) -> anyhow::Result<ChatCompletionChoice> {
    let req = tool_completion_request(prompt, "");
    let mut res = llm.chat_completion(req).await?;

    let chiose = res
        .choices
        .pop()
        .ok_or_else(|| anyhow!("expect at least one chioce"))?;

    Ok(chiose)
}

async fn write_code(llm: &LlmSdk, args: WriteCodeArgs) -> anyhow::Result<WriteCodeResult> {
    let messages = vec![
        ChatCompletionMessage::new_system(
            "I'm an expert on coding, I'll write code for you in markdown format based on your prompt",
            "Ava",
        ),
        ChatCompletionMessage::new_user(args.prompt, ""),
    ];

    let md = chat_completion(llm, messages).await?;
    let content = markdown_to_html(&md, &comrak::ComrakOptions::default());
    Ok(WriteCodeResult::new(content))
}

async fn answer(llm: &LlmSdk, args: AnswerArgs) -> anyhow::Result<String> {
    let messages = vec![
        ChatCompletionMessage::new_system("I can help answer anything you'd like to chat", "Ava"),
        ChatCompletionMessage::new_user(args.prompt, ""),
    ];

    Ok(chat_completion(llm, messages).await?)
}

async fn chat_completion(
    llm: &LlmSdk,
    messages: Vec<ChatCompletionMessage>,
) -> anyhow::Result<String> {
    let req = ChatCompletionRequest::new(messages);

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

async fn draw_image(
    llm: &LlmSdk,
    device_id: &str,
    args: DrawImageArgs,
) -> anyhow::Result<DrawImageResult> {
    let req = CreateImageRequestBuilder::default()
        .prompt(args.prompt)
        .response_format(ImageResponseFormat::B64Json)
        .build()
        .unwrap();

    let mut ret = llm.create_image(req).await?;
    let img = ret
        .data
        .pop()
        .ok_or_else(|| anyhow!("expect at least one data"))?;
    let data = STANDARD.decode(img.b64_json.unwrap())?;
    let uuid = Uuid::new_v4().to_string();
    let path = image_path(device_id, &uuid);
    if let Some(parent) = path.parent() {
        // 父级路径没有创建就创建它
        if !parent.exists() {
            fs::create_dir_all(parent).await?
        }
    }
    fs::write(&path, data).await?;
    Ok(DrawImageResult::new(
        image_url(device_id, &uuid),
        img.revised_prompt,
    ))
}

async fn process(
    event_sender: &broadcast::Sender<AssistantEvent>,
    device_id: &str,
    llm: &LlmSdk,
    mut data: Multipart,
) -> anyhow::Result<()> {
    let id = Uuid::new_v4().to_string();
    event_sender.send(in_audio_upload()).unwrap();
    let Some(field) = data.next_field().await? else {
        return Err(anyhow!("expected an audio field"))?;
    };

    let data = match field.name() {
        Some(name) if name == "audio" => field.bytes().await?,
        _ => return Err(anyhow!("expected an audio field"))?,
    };

    event_sender.send(in_transcrition())?;
    event_sender.send(ChatInputSkeletonEvent::new(&id).into())?;

    let text = transcript(llm, &data).await?;
    event_sender.send(ChatInputEvent::new(&id, &text).into())?;

    event_sender.send(in_thinking())?;
    event_sender.send(ChatReplySkeletonEvent::new(&id).into())?;

    let chioce = chat_completion_with_tools(llm, &text).await?;
    match chioce.finish_reason {
        llm_sdk::chat_completion::FinishReason::Stop => {
            let output = chioce
                .message
                .content
                .ok_or_else(|| anyhow!("expect content but no content available"))?;
            event_sender.send(in_speech())?;
            let ret = SpeechResult::new_text_only(&output);
            event_sender.send(ChatReplyEvent::new(&id, ret).into())?;

            let ret = speech(llm, &device_id, &output).await?;
            event_sender.send(complete())?;
            event_sender.send(ChatReplyEvent::new(id, ret).into())?;
        }

        llm_sdk::chat_completion::FinishReason::ToolCalls => {
            let tool_call = &chioce.message.tool_calls[0].function;
            match AssistantTool::from_str(&tool_call.name) {
                Ok(v) if v == AssistantTool::DrawImage => {
                    let args: DrawImageArgs = serde_json::from_str(&tool_call.arguments)?;

                    event_sender.send(in_draw_image())?;
                    let ret = DrawImageResult::new("", &args.prompt);
                    event_sender.send(ChatReplyEvent::new(&id, ret).into())?;

                    let ret = draw_image(llm, device_id, args).await?;
                    event_sender.send(complete())?;
                    event_sender.send(ChatReplyEvent::new(&id, ret).into())?;
                }
                Ok(v) if v == AssistantTool::WriteCode => {
                    event_sender.send(in_write_code())?;
                    let ret = write_code(llm, serde_json::from_str(&tool_call.arguments)?).await?;

                    event_sender.send(complete())?;
                    event_sender.send(ChatReplyEvent::new(&id, ret).into())?;
                }
                Ok(v) if v == AssistantTool::Answer => {
                    event_sender.send(in_chat_completion())?;
                    let output = answer(llm, serde_json::from_str(&tool_call.arguments)?).await?;
                    event_sender.send(complete())?;
                    let ret = SpeechResult::new_text_only(&output);
                    event_sender.send(ChatReplyEvent::new(&id, ret).into())?;

                    event_sender.send(in_speech())?;
                    let ret = speech(llm, device_id, &output).await?;
                    event_sender.send(complete())?;
                    event_sender.send(ChatReplyEvent::new(&id, ret).into())?;
                }
                _ => {
                    bail!("no proper tool found")
                }
            }
        }
        _ => {
            bail!("stop reason not supported")
        }
    }
    Ok(())
}

fn in_audio_upload() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::UploadAudio).into()
}

fn in_transcrition() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::Transcrition).into()
}

fn in_thinking() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::Thinking).into()
}

fn in_chat_completion() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::ChatCompletion).into()
}

fn in_speech() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::Speech).into()
}

fn in_draw_image() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::DrawImage).into()
}

fn in_write_code() -> AssistantEvent {
    SignalEvent::Processing(AssistantStep::WriteCode).into()
}

fn error(msg: impl Into<String>) -> AssistantEvent {
    SignalEvent::Error(msg.into()).into()
}

fn complete() -> AssistantEvent {
    SignalEvent::Complete.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_render() {
        let event: String = error("error").into();
        assert_eq!(event, "\n<p class=\"text-red-800\">Error: error</p>\n")
    }
}
