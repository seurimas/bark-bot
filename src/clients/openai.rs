use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use openai_api_rs::v1::{
    api::OpenAIClient,
    chat_completion::{
        ChatCompletionMessage, ChatCompletionRequest, ChatCompletionResponse, Content, MessageRole,
        Tool, ToolCallFunction, ToolType,
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

impl From<BarkChat> for ChatCompletionRequest {
    fn from(chat: BarkChat) -> Self {
        let mut combined: Vec<ChatCompletionMessage> = vec![];
        let mut combined_message = ChatCompletionMessage {
            role: MessageRole::user,
            content: Content::Text("".to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        };
        for message in chat.messages {
            if let Some(top) = combined.last_mut() {
                if let Some(text_content) = message.text_content() {
                    if matches!(top.role, MessageRole::user)
                        == matches!(message.role, BarkRole::User)
                    {
                        push_content(&mut top.content, text_content);
                        continue;
                    } else if matches!(top.role, MessageRole::assistant)
                        == matches!(message.role, BarkRole::Assistant)
                    {
                        push_content(&mut top.content, text_content);
                        continue;
                    } else if matches!(top.role, MessageRole::system)
                        == matches!(message.role, BarkRole::System)
                    {
                        push_content(&mut top.content, text_content);
                        continue;
                    }
                    if !content_empty(&combined_message.content) {
                        combined.push(combined_message.clone());
                        combined_message = ChatCompletionMessage {
                            role: MessageRole::user,
                            content: Content::Text(text_content.to_string()),
                            name: None,
                            tool_calls: None,
                            tool_call_id: None,
                        };
                        combined_message.role = match message.role {
                            BarkRole::User => MessageRole::user,
                            BarkRole::Assistant => MessageRole::assistant,
                            BarkRole::System => MessageRole::system,
                            BarkRole::Tool => MessageRole::tool,
                        };
                    }
                } else if let (Some(ref mut tool_calls), Some(tool_call)) =
                    (&mut top.tool_calls, message.tool_call())
                {
                    tool_calls.push(openai_api_rs::v1::chat_completion::ToolCall {
                        id: tool_call.id.clone(),
                        r#type: "function".to_string(),
                        function: ToolCallFunction {
                            name: Some(tool_call.function_name.clone()),
                            arguments: tool_call.arguments.clone(),
                        },
                    });
                    continue;
                } else if let Some(tool_call) = message.tool_call() {
                    combined_message.tool_calls =
                        Some(vec![openai_api_rs::v1::chat_completion::ToolCall {
                            id: tool_call.id.clone(),
                            r#type: "function".to_string(),
                            function: ToolCallFunction {
                                name: Some(tool_call.function_name.clone()),
                                arguments: tool_call.arguments.clone(),
                            },
                        }]);
                    continue;
                }
            } else if let Some(text_content) = message.text_content() {
                push_content(&mut combined_message.content, text_content);
                combined_message.role = match message.role {
                    BarkRole::User => MessageRole::user,
                    BarkRole::Assistant => MessageRole::assistant,
                    BarkRole::System => MessageRole::system,
                    BarkRole::Tool => MessageRole::tool,
                };
            }
        }
        if !content_empty(&combined_message.content) {
            combined.push(combined_message.clone());
        }
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
