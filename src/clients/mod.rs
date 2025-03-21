use serde::{Deserialize, Serialize};

mod openai;
pub use openai::*;
mod ollama;
pub use ollama::*;
mod mcp;
pub use mcp::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BarkMessage {
    pub role: BarkRole,
    pub content: BarkContent,
}

impl BarkMessage {
    pub fn text_content(&self) -> Option<&String> {
        match &self.content {
            BarkContent::Text(text) => Some(text),
            BarkContent::ToolCall(_) => None,
        }
    }

    pub fn tool_call(&self) -> Option<&BarkToolCall> {
        match &self.content {
            BarkContent::Text(_) => None,
            BarkContent::ToolCall(request) => Some(request),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BarkContent {
    Text(String),
    ToolCall(BarkToolCall),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BarkRole {
    System,
    Assistant,
    User,
    Tool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Choice {
    pub index: usize,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BarkToolCall {
    pub id: String,
    pub function_name: String,
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BarkToolCallResponse {
    pub id: String,
    pub function_name: String,
    pub arguments: Option<String>,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum BarkResponse {
    Chat {
        choices: Vec<Choice>,
        usage: Option<u32>,
    },
    ToolCalls {
        calls: Vec<BarkToolCall>,
        usage: Option<u32>,
    },
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
            model: "BARK CHAT MODEL NOT OVERRIDEN".to_string(),
            temperature: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BarkTool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

impl BarkTool {
    pub fn debug_tool() -> Self {
        Self {
            name: "debug_tool".to_string(),
            description: "Debugging tool - Only use if prompted".to_string(),
            parameters: serde_json::json!({}),
        }
    }
}
