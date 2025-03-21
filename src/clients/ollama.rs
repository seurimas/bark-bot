use std::collections::HashMap;

use crate::bt::{AiModelConfig, BarkModelConfig};

use super::{BarkChat, BarkResponse, BarkRole, BarkTool};

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
            mcp_services: HashMap::new(),
            embedding_model,
        })
    } else {
        None
    }
}

pub fn ollama_get_bark_response(
    client: &ollama_rs::Ollama,
    chat: BarkChat,
    tools: &Vec<BarkTool>,
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
        Self::Chat {
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
        let open_ai_chat: openai_api_rs::v1::chat_completion::ChatCompletionRequest = chat.into();
        let ollama_chat = serde_json::from_str(&serde_json::to_string(&open_ai_chat).unwrap())
            .expect("Failed to convert to Ollama chat request");
        ollama_chat
    }
}
