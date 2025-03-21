use std::collections::HashMap;

use ollama_rs::generation::options::GenerationOptions;

use crate::bt::{AiModelConfig, BarkModelConfig};

use super::{BarkChat, BarkResponse, BarkRole};

pub fn ollama_get_from_env() -> Option<BarkModelConfig> {
    if let Ok(host) = std::env::var("OLLAMA_HOST") {
        let mut models = HashMap::new();
        let model = std::env::var("MODEL_NAME").unwrap_or("deepseek-r1:14b".to_string());
        models.insert(
            "default".to_string(),
            AiModelConfig {
                model_name: model.clone(),
                api_key: "".to_string(),
                url: host.clone(),
                temperature: None,
            },
        );
        let embedding_model = (
            std::env::var("EMBEDDING_MODEL_NAME").unwrap_or("BAAI/bge-small-en-v1.5".to_string()),
            "".to_string(),
            host,
        );

        Some(BarkModelConfig {
            openai_models: HashMap::new(),
            ollama_models: models,
            embedding_model,
        })
    } else {
        None
    }
}

pub fn ollama_get_bark_response(
    client: &ollama_rs::Ollama,
    chat: BarkChat,
) -> Result<BarkResponse, String> {
    futures::executor::block_on(async {
        client
            .send_chat_messages(chat.into())
            .await
            .map(|response| response.into())
            .map_err(|e| format!("Error: {:?}", e))
    })
}

impl From<ollama_rs::generation::chat::ChatMessageResponse> for BarkResponse {
    fn from(response: ollama_rs::generation::chat::ChatMessageResponse) -> Self {
        Self {
            choices: vec![super::Choice {
                index: 0,
                value: response.message.content,
            }],
            usage: response
                .final_data
                .map(|data| (data.eval_count + data.prompt_eval_count) as u32),
        }
    }
}

impl From<BarkChat> for ollama_rs::generation::chat::request::ChatMessageRequest {
    fn from(chat: BarkChat) -> Self {
        let mut combined: Vec<ollama_rs::generation::chat::ChatMessage> = vec![];
        let mut combined_message = ollama_rs::generation::chat::ChatMessage {
            role: ollama_rs::generation::chat::MessageRole::User,
            content: "".to_string(),
            tool_calls: vec![],
            images: None,
        };
        if chat.messages.is_empty() || chat.messages[0].role != BarkRole::System {
            combined.push(ollama_rs::generation::chat::ChatMessage {
                role: ollama_rs::generation::chat::MessageRole::System,
                content: "Respond helpfully and concisely to queries. For very complicated queries, think it through first. Otherwise, just answer.".to_string(),
                tool_calls: vec![],
                images: None,
            });
        }
        for message in chat.messages {
            if let Some(top) = combined.last_mut() {
                if matches!(top.role, ollama_rs::generation::chat::MessageRole::User)
                    == matches!(message.role, BarkRole::User)
                {
                    top.content.push_str(&message.content);
                    continue;
                } else if matches!(
                    top.role,
                    ollama_rs::generation::chat::MessageRole::Assistant
                ) == matches!(message.role, BarkRole::Assistant)
                {
                    top.content.push_str(&message.content);
                    continue;
                } else if matches!(top.role, ollama_rs::generation::chat::MessageRole::System)
                    == matches!(message.role, BarkRole::System)
                {
                    top.content.push_str(&message.content);
                    continue;
                }
            }
            combined_message.role = match message.role {
                BarkRole::User => ollama_rs::generation::chat::MessageRole::User,
                BarkRole::Assistant => ollama_rs::generation::chat::MessageRole::Assistant,
                BarkRole::System => ollama_rs::generation::chat::MessageRole::System,
            };
            combined.push(combined_message);
            combined_message = ollama_rs::generation::chat::ChatMessage {
                role: ollama_rs::generation::chat::MessageRole::User,
                content: "".to_string(),
                tool_calls: vec![],
                images: None,
            };
        }
        if let Some(temperature) = chat.temperature {
            ollama_rs::generation::chat::request::ChatMessageRequest::new(chat.model, combined)
                .options(GenerationOptions::default().temperature(temperature))
        } else {
            ollama_rs::generation::chat::request::ChatMessageRequest::new(chat.model, combined)
        }
    }
}
