use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use openai_api_rs::v1::{
    api::OpenAIClient,
    chat_completion::{
        ChatCompletionMessage, ChatCompletionRequest, ChatCompletionResponse, Content, MessageRole,
        Tool, ToolCall, ToolCallFunction, ToolType,
    },
    embedding::EmbeddingRequest,
    types::{Function, FunctionParameters, JSONSchemaDefine, JSONSchemaType},
};
use serde_json::Value;

use crate::bt::{AiModelConfig, BarkModelConfig};

use super::{BarkChat, BarkResponse, BarkRole, BarkTool, BarkToolCall};

#[derive(Clone)]
pub struct OpenAI(Arc<Mutex<openai_api_rs::v1::api::OpenAIClient>>);

impl OpenAI {
    pub fn new(api_key: &String, url: &String) -> Self {
        let client = OpenAIClient::builder()
            .with_api_key(api_key.clone())
            .with_endpoint(url.clone())
            .build()
            .unwrap();
        Self(Arc::new(Mutex::new(client)))
    }

    pub async fn embeddings_create(
        &self,
        model: &str,
        input: Vec<String>,
    ) -> Result<openai_api_rs::v1::embedding::EmbeddingResponse, String> {
        let mut client = self.0.lock().unwrap();
        let request = EmbeddingRequest::new(model.to_string(), input);
        client
            .embedding(request)
            .await
            .map_err(|e| format!("Error: {:?}", e))
    }
}

impl std::fmt::Debug for OpenAI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OpenAI").finish()
    }
}

pub fn openai_get_from_env() -> Option<BarkModelConfig> {
    if let (Ok(api_key), Some(url)) = (
        &std::env::var("OPENAI_API_KEY"),
        &std::env::var("OPENAI_URL").ok(),
    ) {
        let mut models = HashMap::new();
        let model = std::env::var("MODEL_NAME").unwrap_or("mistral-nemo".to_string());
        models.insert(
            "default".to_string(),
            AiModelConfig {
                model_name: model.clone(),
                api_key: api_key.clone(),
                url: url.clone(),
                temperature: None,
            },
        );
        let embedding_model = (
            std::env::var("EMBEDDING_MODEL_NAME").unwrap_or("BAAI/bge-small-en-v1.5".to_string()),
            api_key.clone(),
            url.clone(),
        );

        Some(BarkModelConfig {
            openai_models: models,
            ollama_models: HashMap::new(),
            mcp_services: HashMap::new(),
            embedding_model,
        })
    } else {
        None
    }
}

pub async fn openai_get_bark_response(
    client: &OpenAI,
    chat: BarkChat,
    tools: &Vec<BarkTool>,
) -> Result<BarkResponse, String> {
    let mut client = client.0.lock().unwrap();
    let chat_request: ChatCompletionRequest = chat.into();
    let chat_request = chat_request.tools(
        tools
            .iter()
            .cloned()
            .map(|t| t.into())
            .collect::<Vec<Tool>>(),
    );
    println!("Chat request: {:?}", chat_request);
    client
        .chat_completion(chat_request)
        .await
        .map(|response| response.into())
        .map_err(|e| format!("Error: {:?}", e))
}

impl From<ChatCompletionResponse> for BarkResponse {
    fn from(mut response: ChatCompletionResponse) -> Self {
        println!("Response: {:?}", response);
        let Some(choice) = response.choices.pop() else {
            println!("Empty response: {:?}", response);
            return BarkResponse::Chat {
                choices: vec![],
                usage: None,
            };
        };
        if let Some(content) = choice.message.content {
            return BarkResponse::Chat {
                choices: vec![super::Choice {
                    index: 0,
                    value: content,
                }],
                usage: Some(response.usage.total_tokens as u32),
            };
        } else if let Some(tool_calls) = choice.message.tool_calls {
            return BarkResponse::ToolCalls {
                calls: tool_calls
                    .iter()
                    .map(|call| call.into())
                    .collect::<Vec<BarkToolCall>>(),
                usage: Some(response.usage.total_tokens as u32),
            };
        } else {
            println!("Empty response: {:?}", response);
            return BarkResponse::Chat {
                choices: vec![],
                usage: None,
            };
        }
    }
}

fn push_content(content: &mut Content, string: &str) {
    let Content::Text(ref mut text) = content else {
        panic!("Expected text content for user message");
    };
    text.push_str(string);
}

fn content_empty(content: &Content) -> bool {
    match content {
        Content::Text(text) => text.is_empty(),
        _ => true,
    }
}

impl From<BarkRole> for MessageRole {
    fn from(value: BarkRole) -> Self {
        match value {
            BarkRole::System => MessageRole::system,
            BarkRole::Assistant => MessageRole::assistant,
            BarkRole::User => MessageRole::user,
            BarkRole::Tool => MessageRole::tool,
        }
    }
}

impl From<BarkChat> for ChatCompletionRequest {
    fn from(chat: BarkChat) -> Self {
        let mut combined: Vec<ChatCompletionMessage> = vec![];
        println!("Chat messages: {:?}", chat.messages);
        for message in chat.messages {
            if let Some(text_content) = message.text_content() {
                if combined.is_empty() {
                    combined.push(ChatCompletionMessage {
                        role: message.role.into(),
                        content: Content::Text(text_content.clone()),
                        tool_calls: None,
                        name: None,
                        tool_call_id: None,
                    });
                } else {
                    let Some(top) = combined.last_mut() else {
                        panic!("Expected at least one message in combined");
                    };
                    if top.tool_calls.is_some() || top.tool_call_id.is_some() {
                        combined.push(ChatCompletionMessage {
                            role: message.role.into(),
                            content: Content::Text(text_content.clone()),
                            tool_calls: None,
                            name: None,
                            tool_call_id: message.tool_id().cloned(),
                        });
                    } else if matches!(top.role, MessageRole::user)
                        == matches!(message.role, BarkRole::User)
                    {
                        push_content(&mut top.content, text_content);
                    } else if matches!(top.role, MessageRole::assistant)
                        == matches!(message.role, BarkRole::Assistant)
                    {
                        push_content(&mut top.content, text_content);
                    } else if matches!(top.role, MessageRole::system)
                        == matches!(message.role, BarkRole::System)
                    {
                        push_content(&mut top.content, text_content);
                    } else if matches!(top.role, MessageRole::tool)
                        == matches!(message.role, BarkRole::Tool)
                    {
                        push_content(&mut top.content, text_content);
                    } else {
                        combined.push(ChatCompletionMessage {
                            role: message.role.into(),
                            content: Content::Text(text_content.clone()),
                            tool_calls: None,
                            name: None,
                            tool_call_id: message.tool_id().cloned(),
                        });
                    }
                }
            } else if let Some(tool_call) = message.tool_call() {
                println!("Pushing tool call: {:?}", tool_call);
                combined.push(ChatCompletionMessage {
                    role: message.role.into(),
                    content: Content::Text("".to_string()),
                    name: None,
                    tool_call_id: Some(tool_call.id.clone()),
                    tool_calls: Some(vec![ToolCall {
                        id: tool_call.id.clone(),
                        r#type: "function".to_string(),
                        function: ToolCallFunction {
                            name: Some(tool_call.function_name.clone()),
                            arguments: tool_call.arguments.clone(),
                        },
                    }]),
                });
            }
        }
        println!("Combined messages: {:?}", combined);
        ChatCompletionRequest {
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
            parallel_tool_calls: None,
            response_format: None,
            tools: None,
            seed: None,
            tool_choice: None,
        }
    }
}

impl From<BarkTool> for Tool {
    fn from(tools: BarkTool) -> Self {
        Tool {
            r#type: ToolType::Function,
            function: Function {
                name: tools.name,
                description: Some(tools.description),
                parameters: get_parameters_from_value(tools.parameters),
            },
        }
    }
}

impl From<&openai_api_rs::v1::chat_completion::ToolCall> for BarkToolCall {
    fn from(tool_call: &openai_api_rs::v1::chat_completion::ToolCall) -> Self {
        BarkToolCall {
            id: tool_call.id.clone(),
            function_name: tool_call
                .function
                .name
                .clone()
                .unwrap_or("UNNAMED".to_string()),
            arguments: tool_call.function.arguments.clone(),
        }
    }
}

fn get_parameters_from_value(value: Value) -> FunctionParameters {
    FunctionParameters {
        schema_type: JSONSchemaType::Object,
        properties: Some(HashMap::new()),
        required: Some(Vec::new()),
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::prelude::*;

    #[test]
    fn test_bark_chat_tool_call() {
        let messages = vec![
            BarkMessage {
                role: BarkRole::User,
                content: BarkContent::Text(
                    "Call the debug function. That's all I want you to do.".to_string(),
                ),
            },
            BarkMessage {
                role: BarkRole::Assistant,
                content: BarkContent::ToolCall(BarkToolCall {
                    id: "call_1NBig8Eb6l2nuDBqrZzFVgu9".to_string(),
                    function_name: "debug_tool".to_string(),
                    arguments: Some("{}".to_string()),
                }),
            },
            BarkMessage {
                role: BarkRole::Tool,
                content: BarkContent::Text("Successful! Please tell me you love me.".to_string()),
            },
        ];
        let chat = BarkChat {
            messages,
            model: "gpt-4".to_string(),
            temperature: None,
        };
        let chat_request: ChatCompletionRequest = chat.into();
        println!("Chat request: {:?}", chat_request);
        assert_eq!(chat_request.messages.len(), 3);
    }
}
