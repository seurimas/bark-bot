use rmcp::{
    model::{
        CallToolRequestParam, CallToolResult, ClientCapabilities, ClientInfo, Content, ErrorData,
        Implementation, ListToolsResult, Tool,
    },
    serve_client,
    service::RunningService,
    transport::{SseTransport, TokioChildProcess},
    ClientHandler, Error, RoleClient, ServiceExt,
};
use serde_json::Value;
use std::{any, collections::HashMap, future::Future, pin::Pin, sync::Arc};
use tokio::{process::Command, runtime::Handle, sync::Mutex, task::JoinHandle};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tower::{timeout::Timeout, Service};

use super::{apply_tool_filters, BarkTool, BarkToolCall, BarkToolCallResponse};

pub trait McpServiceClient: Send + Sync {
    // fn call_mcp(
    //     &mut self,
    //     tool_name: &str,
    //     arguments: Value,
    // ) -> JoinHandle<core::result::Result<CallToolResult, Error>>;

    // fn list_mcp_tools(&self) -> JoinHandle<Result<ListToolsResult, Error>>;
}

impl<S> McpServiceClient for RunningService<RoleClient, S>
where
    S: ClientHandler,
{
    // fn call_mcp(
    //     &mut self,
    //     tool_name: &str,
    //     arguments: Value,
    // ) -> JoinHandle<core::result::Result<CallToolResult, Error>> {
    //     let tool_request = CallToolRequestParam {
    //         name: tool_name.to_string().into(),
    //         arguments: arguments.as_object().cloned(),
    //     };
    //     tokio::spawn(async {
    //         self.call_tool(tool_request)
    //             .await
    //             .map_err(|e| Error::internal_error(e.to_string(), None))
    //     })
    // }

    // fn list_mcp_tools(&self) -> JoinHandle<Result<ListToolsResult, Error>> {
    //     tokio::spawn(async {
    //         self.list_tools(None)
    //             .await
    //             .map_err(|e| Error::internal_error(e.to_string(), None))
    //     })
    // }
}

fn default_timeout_seconds() -> f32 {
    30.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServiceConfig {
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: f32,
    #[serde(default)]
    pub tool_filters: Vec<String>,
}

pub async fn initialize_stdio_mcp_service(
    name: &str,
    config: &McpServiceConfig,
) -> Result<Box<dyn McpServiceClient>> {
    let transport = TokioChildProcess::new(
        Command::new(&config.command)
            .args(&config.args)
            .envs(&config.env),
    )?;

    let client_info = ClientInfo {
        capabilities: ClientCapabilities::default(),
        protocol_version: Default::default(),
        client_info: Implementation {
            name: name.to_string(),
            version: "1.0.0".to_string(),
        },
    };
    let client = client_info.serve(transport).await?;

    Ok(Box::new(client))
}

async fn initialize_sse_mcp_service(name: &str, host: &str) -> Result<Box<dyn McpServiceClient>> {
    let transport = SseTransport::start(host).await?;
    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: name.to_string(),
            version: "1.0.0".to_string(),
        },
    };
    let client = client_info.serve(transport).await?;
    Ok(Box::new(client))
}

pub async fn initialize_mcp_service_map(
    config: &HashMap<String, McpServiceConfig>,
) -> HashMap<String, Arc<Mutex<Box<dyn McpServiceClient>>>> {
    let mut mcp_services = HashMap::new();
    for (name, service_config) in config.iter() {
        match initialize_stdio_mcp_service(name, service_config).await {
            Ok(client) => {
                mcp_services.insert(name.clone(), Arc::new(Mutex::new(client)));
            }
            Err(e) => {
                eprintln!("Failed to initialize service {name}: {e}");
            }
        }
    }
    mcp_services
}

pub async fn initialize_sse_mcp_service_map(
    hosts: &HashMap<String, String>,
) -> HashMap<String, Arc<Mutex<Box<dyn McpServiceClient>>>> {
    let mut mcp_services = HashMap::new();
    for (name, host) in hosts.iter() {
        match initialize_sse_mcp_service(name, host).await {
            Ok(client) => {
                mcp_services.insert(name.clone(), Arc::new(Mutex::new(client)));
            }
            Err(e) => {
                eprintln!("Failed to initialize service {name}: {e}");
            }
        }
    }
    mcp_services
}

pub async fn initialize_mcp_tool_map(
    clients: &HashMap<String, Arc<Mutex<Box<dyn McpServiceClient>>>>,
    filters: &HashMap<String, Vec<String>>,
) -> HashMap<String, BarkTool> {
    let mut mcp_tools = HashMap::new();
    for (name, client) in clients.iter() {
        let client = client.lock().await;
        // let tools = client.list_mcp_tools().await.unwrap_or(Err(|e| {
        //     eprintln!("Failed to list tools for service {name}: {e}");
        //     Err(e)
        // }));
        let tools: Result<ListToolsResult, String> =
            Err("Mocked error for listing tools".to_string()); // Mocked for example purposes
        match tools {
            Ok(tool_list) => {
                for tool in tool_list.tools.iter() {
                    let tool_name = tool.name.clone();
                    let tool_name = format!("{name}__{tool_name}");
                    if !apply_tool_filters(filters.get(name).unwrap_or(&Vec::new()), &tool_name) {
                        continue;
                    }
                    let tool_description = tool.description.clone();
                    mcp_tools.insert(
                        tool_name.clone(),
                        BarkTool {
                            name: tool_name,
                            description: tool_description.to_string(),
                            parameters: serde_json::Value::Object((*tool.input_schema).clone()),
                        },
                    );
                }
            }
            Err(e) => {
                eprintln!("Failed to list tools for service {name}: {e}");
            }
        }
    }
    mcp_tools
}

impl From<Tool> for BarkTool {
    fn from(tool: Tool) -> Self {
        Self {
            name: tool.name.to_string(),
            description: tool.description.to_string(),
            parameters: serde_json::Value::Object((*tool.input_schema).clone()),
        }
    }
}

impl From<BarkTool> for Tool {
    fn from(tool: BarkTool) -> Self {
        Self {
            name: tool.name.into(),
            description: tool.description.into(),
            input_schema: match tool.parameters {
                serde_json::Value::Object(obj) => Arc::new(obj),
                _ => Arc::new(serde_json::Map::new()),
            },
        }
    }
}

impl BarkToolCallResponse {
    pub fn try_parse(
        call: &BarkToolCall,
        mut value: CallToolResult,
    ) -> std::result::Result<Self, String> {
        if value.is_error.unwrap_or(false) {
            Err(format!("Tool call error: {:?}", value.content))
        } else {
            let Some(top) = value.content.pop() else {
                return Err(format!("Empty tool call response"));
            };
            match top.as_text() {
                Some(text) => Ok(BarkToolCallResponse {
                    id: call.id.clone(),
                    function_name: call.function_name.clone(),
                    arguments: call.arguments.clone(),
                    result: Some(text.text.clone()),
                }),
                _ => Err(format!("Unsupported tool response type")),
            }
        }
    }
}
