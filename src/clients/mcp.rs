use rmcp::{
    model::{
        CallToolRequestParam, CallToolResult, ClientCapabilities, ClientInfo, Implementation,
        ListToolsResult, Tool,
    },
    service::RunningService,
    transport::{SseTransport, TokioChildProcess},
    RoleClient, ServiceExt,
};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tokio::{process::Command, sync::Mutex, task::JoinHandle};

use anyhow::{anyhow, Error, Result};
use serde::{Deserialize, Serialize};

use super::{apply_tool_filters, BarkTool, BarkToolCall, BarkToolCallResponse};

pub trait McpServiceClient: Send + Sync {
    fn call_mcp(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> JoinHandle<core::result::Result<CallToolResult, Error>>;

    fn list_mcp_tools(&self) -> JoinHandle<Result<ListToolsResult, Error>>;
}

#[derive(Clone)]
pub struct RunningServiceClient {
    service: Arc<Mutex<RunningService<RoleClient, rmcp::model::InitializeRequestParam>>>,
}

impl From<RunningService<RoleClient, rmcp::model::InitializeRequestParam>>
    for RunningServiceClient
{
    fn from(service: RunningService<RoleClient, rmcp::model::InitializeRequestParam>) -> Self {
        Self {
            service: Arc::new(Mutex::new(service)),
        }
    }
}

impl McpServiceClient for RunningServiceClient {
    fn call_mcp(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> JoinHandle<Result<CallToolResult, Error>> {
        let tool_request = CallToolRequestParam {
            name: tool_name.to_string().into(),
            arguments: arguments.as_object().cloned(),
        };
        let service = self.service.clone();
        tokio::spawn(async move {
            let service = service.lock().await;
            service
                .call_tool(tool_request)
                .await
                .map_err(|e| anyhow!("Failed to call tool: {}", e))
        })
    }

    fn list_mcp_tools(&self) -> JoinHandle<Result<ListToolsResult, Error>> {
        let service = self.service.clone();
        tokio::spawn(async move {
            let service = service.lock().await;
            service
                .list_tools(None)
                .await
                .map_err(|e| anyhow!("Failed to call tool: {}", e))
        })
    }
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
) -> Result<RunningServiceClient> {
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

    Ok(client.into())
}

async fn initialize_sse_mcp_service(name: &str, host: &str) -> Result<RunningServiceClient> {
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
    Ok(client.into())
}

pub async fn initialize_mcp_service_map(
    config: &HashMap<String, McpServiceConfig>,
) -> HashMap<String, RunningServiceClient> {
    let mut mcp_services = HashMap::new();
    for (name, service_config) in config.iter() {
        match initialize_stdio_mcp_service(name, service_config).await {
            Ok(client) => {
                mcp_services.insert(name.clone(), client);
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
) -> HashMap<String, RunningServiceClient> {
    let mut mcp_services = HashMap::new();
    for (name, host) in hosts.iter() {
        match initialize_sse_mcp_service(name, host).await {
            Ok(client) => {
                mcp_services.insert(name.clone(), client);
            }
            Err(e) => {
                eprintln!("Failed to initialize service {name}: {e}");
            }
        }
    }
    mcp_services
}

pub async fn initialize_mcp_tool_map(
    clients: &HashMap<String, RunningServiceClient>,
    filters: &HashMap<String, Vec<String>>,
) -> HashMap<String, BarkTool> {
    let mut mcp_tools = HashMap::new();
    for (name, client) in clients.iter() {
        let tools = client
            .list_mcp_tools()
            .await
            .unwrap_or(Err(Error::msg("Failed to list tools")));
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
