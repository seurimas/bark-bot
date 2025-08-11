use std::collections::HashMap;

use ollama_rs::{
    generation::{
        chat::{request::ChatMessageRequest, ChatMessage, MessageRole},
        tools::{ToolCall, ToolInfo},
    },
    models::ModelOptions,
};
use openai_api_rs::v1::chat_completion::Tool;
use serde::{Deserialize, Serialize};

use crate::{
    bt::{AiModelConfig, BarkModelConfig},
    clients::{BarkToolCall, McpAndTreeConfig},
};

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
            host,
            None,
        );

        Some(BarkModelConfig {
            openai_models: HashMap::new(),
            ollama_models: models,
            tools: McpAndTreeConfig::default(),
            embedding_model,
        })
    } else {
        None
    }
}

pub async fn ollama_get_bark_response(
    client: &ollama_rs::Ollama,
    chat: BarkChat,
    tools: &Vec<BarkTool>,
) -> Result<BarkResponse, String> {
    let mut chat_request: ollama_rs::generation::chat::request::ChatMessageRequest = chat.into();
    chat_request.tools = tools.iter().map(|tool| tool.clone().into()).collect();
    client
        .send_chat_messages(chat_request)
        .await
        .map(|response| response.into())
        .map_err(|e| format!("Error: {:?}", e))
}

impl From<ollama_rs::generation::chat::ChatMessageResponse> for BarkResponse {
    fn from(response: ollama_rs::generation::chat::ChatMessageResponse) -> Self {
        if !response.message.tool_calls.is_empty() {
            return BarkResponse::ToolCalls {
                calls: response
                    .message
                    .tool_calls
                    .iter()
                    .map(|call| call.into())
                    .collect::<Vec<BarkToolCall>>(),
                usage: response
                    .final_data
                    .map(|data| (data.eval_count + data.prompt_eval_count) as u32),
            };
        }
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

fn push_content(content: &mut String, string: &str) {
    content.push_str(string);
}

impl From<BarkRole> for MessageRole {
    fn from(role: BarkRole) -> Self {
        match role {
            BarkRole::User => MessageRole::User,
            BarkRole::Assistant => MessageRole::Assistant,
            BarkRole::System => MessageRole::System,
            BarkRole::Tool => MessageRole::Tool,
        }
    }
}

impl From<BarkChat> for ollama_rs::generation::chat::request::ChatMessageRequest {
    fn from(chat: BarkChat) -> Self {
        let mut combined: Vec<ChatMessage> = vec![];
        for message in chat.messages {
            if let Some(text_content) = message.text_content() {
                if combined.is_empty() {
                    combined.push(ChatMessage {
                        role: message.role.into(),
                        content: text_content.clone(),
                        tool_calls: vec![],
                        images: None,
                    });
                } else {
                    let Some(top) = combined.last_mut() else {
                        panic!("Expected at least one message in combined");
                    };
                    if !top.tool_calls.is_empty() {
                        combined.push(ChatMessage {
                            role: message.role.into(),
                            content: text_content.clone(),
                            tool_calls: vec![],
                            images: None,
                        });
                    } else if matches!(top.role, MessageRole::User)
                        == matches!(message.role, BarkRole::User)
                    {
                        push_content(&mut top.content, text_content);
                    } else if matches!(top.role, MessageRole::Assistant)
                        == matches!(message.role, BarkRole::Assistant)
                    {
                        push_content(&mut top.content, text_content);
                    } else if matches!(top.role, MessageRole::System)
                        == matches!(message.role, BarkRole::System)
                    {
                        push_content(&mut top.content, text_content);
                    } else if matches!(top.role, MessageRole::Tool)
                        == matches!(message.role, BarkRole::Tool)
                    {
                        push_content(&mut top.content, text_content);
                    } else {
                        combined.push(ChatMessage {
                            role: message.role.into(),
                            content: text_content.clone(),
                            tool_calls: vec![],
                            images: None,
                        });
                    }
                }
            } else if let Some(tool_call) = message.tool_call() {
                combined.push(ChatMessage {
                    role: message.role.into(),
                    content: "".to_string(),
                    tool_calls: vec![MyToolCall::tool_call(
                        tool_call.function_name.clone(),
                        if let Some(args) = &tool_call.arguments {
                            serde_json::from_str(args).expect("Failed to parse tool call arguments")
                        } else {
                            serde_json::Value::Null
                        },
                    )],
                    images: None,
                });
            }
        }
        let mut result = ChatMessageRequest::new(chat.model, combined);
        if let Some(temperature) = chat.temperature {
            result.options = Some(ModelOptions::default().temperature(temperature));
        }
        result
    }
}

impl From<BarkTool> for ToolInfo {
    fn from(value: BarkTool) -> Self {
        let open_ai: Tool = value.into();
        let mut serialized = serde_json::to_string(&open_ai).expect("Failed to serialize tool");
        serialized = serialized.replace("\"type\":\"function\"", "\"type\":\"Function\"");
        let result = serde_json::from_str(&serialized)
            .expect("Failed to convert OpenAI tool to Ollama ToolInfo");
        result
    }
}

// Hack to convert ToolCall to BarkToolCall
#[derive(Serialize, Deserialize)]
pub struct MyToolCall {
    function: MyToolCallFunction,
}
#[derive(Serialize, Deserialize)]
pub struct MyToolCallFunction {
    name: String,
    arguments: serde_json::Value,
}

impl MyToolCall {
    pub fn tool_call(name: String, arguments: serde_json::Value) -> ToolCall {
        let my_tool_call = MyToolCall {
            function: MyToolCallFunction { name, arguments },
        };
        let tool_call_string = serde_json::to_string(&my_tool_call).unwrap();
        serde_json::from_str(&tool_call_string).unwrap()
    }
}

impl From<&ToolCall> for BarkToolCall {
    fn from(tool_call: &ToolCall) -> Self {
        let tool_call_string: String = serde_json::to_string(tool_call).unwrap();
        let MyToolCall { function }: MyToolCall = serde_json::from_str(&tool_call_string).unwrap();
        let MyToolCallFunction { name, arguments } = function;
        BarkToolCall {
            id: "OLLAMA_TOOL_CALL".to_string(),
            function_name: name.clone(),
            arguments: Some(arguments.to_string()),
        }
    }
}
