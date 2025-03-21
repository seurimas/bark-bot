use std::collections::HashMap;

use openai_api_rust::{chat::ChatApi, Auth, OpenAI};

use crate::bt::{AiModelConfig, BarkModelConfig};

use super::{BarkChat, BarkResponse, BarkRole};

pub fn openai_get_from_env() -> Option<BarkModelConfig> {
    if let (Ok(auth), Some(url)) = (Auth::from_env(), &std::env::var("OPENAI_URL").ok()) {
        let mut models = HashMap::new();
        let model = std::env::var("MODEL_NAME").unwrap_or("mistral-nemo".to_string());
        models.insert(
            "default".to_string(),
            AiModelConfig {
                model_name: model.clone(),
                api_key: auth.api_key.clone(),
                url: url.clone(),
                temperature: None,
            },
        );
        let embedding_model = (
            std::env::var("EMBEDDING_MODEL_NAME").unwrap_or("BAAI/bge-small-en-v1.5".to_string()),
            auth.api_key.clone(),
            url.clone(),
        );

        Some(BarkModelConfig {
            openai_models: models,
            ollama_models: HashMap::new(),
            embedding_model,
        })
    } else {
        None
    }
}
pub fn openai_get_bark_response(client: &OpenAI, chat: BarkChat) -> Result<BarkResponse, String> {
    client
        .chat_completion_create(&chat.into())
        .map(|response| response.into())
        .map_err(|e| format!("Error: {:?}", e))
}

impl From<openai_api_rust::completions::Completion> for BarkResponse {
    fn from(response: openai_api_rust::completions::Completion) -> Self {
        Self {
            choices: response
                .choices
                .into_iter()
                .enumerate()
                .map(|(idx, c)| super::Choice {
                    index: idx,
                    value: c.message.unwrap().content,
                })
                .collect(),
            usage: response.usage.total_tokens,
        }
    }
}

impl From<BarkChat> for openai_api_rust::chat::ChatBody {
    fn from(chat: BarkChat) -> Self {
        let mut combined: Vec<openai_api_rust::Message> = vec![];
        let mut combined_message = openai_api_rust::Message {
            role: openai_api_rust::Role::User,
            content: "".to_string(),
        };
        for message in chat.messages {
            if let Some(top) = combined.last_mut() {
                if matches!(top.role, openai_api_rust::Role::User)
                    == matches!(message.role, BarkRole::User)
                {
                    top.content.push_str(&message.content);
                    continue;
                } else if matches!(top.role, openai_api_rust::Role::Assistant)
                    == matches!(message.role, BarkRole::Assistant)
                {
                    top.content.push_str(&message.content);
                    continue;
                } else if matches!(top.role, openai_api_rust::Role::System)
                    == matches!(message.role, BarkRole::System)
                {
                    top.content.push_str(&message.content);
                    continue;
                }
            }
            combined_message.role = match message.role {
                BarkRole::User => openai_api_rust::Role::User,
                BarkRole::Assistant => openai_api_rust::Role::Assistant,
                BarkRole::System => openai_api_rust::Role::System,
            };
            combined.push(combined_message);
            combined_message = openai_api_rust::Message {
                role: openai_api_rust::Role::User,
                content: "".to_string(),
            };
        }
        openai_api_rust::chat::ChatBody {
            frequency_penalty: None,
            logit_bias: None,
            max_tokens: Some(4096),
            messages: combined,
            model: chat.model,
            n: None,
            presence_penalty: None,
            stop: None,
            stream: None,
            temperature: None,
            top_p: None,
            user: None,
        }
    }
}
