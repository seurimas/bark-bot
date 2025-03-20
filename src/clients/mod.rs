use serde::{Deserialize, Serialize};

mod openai;
pub use openai::*;
mod ollama;
pub use ollama::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BarkMessage {
    pub role: BarkRole,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BarkRole {
    System,
    Assistant,
    User,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Choice {
    pub index: usize,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BarkResponse {
    pub choices: Vec<Choice>,
    pub usage: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BarkChat {
    pub messages: Vec<BarkMessage>,
    pub model: String,
    pub temperature: Option<f32>,
}

impl From<Vec<BarkMessage>> for BarkChat {
    fn from(messages: Vec<BarkMessage>) -> Self {
        Self {
            messages,
            model: "dolphin-2.1-mistral-7b.Q4_K_M.gguf".to_string(),
            temperature: None,
        }
    }
}
