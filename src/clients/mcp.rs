use futures::executor::block_on;
use mcp_core::{
    protocol::{CallToolResult, JsonRpcMessage, ListToolsResult},
    Content, Tool,
};
use serde_json::Value;
use std::{
    any,
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use mcp_client::{
    transport::stdio::StdioTransportHandle, ClientCapabilities, ClientInfo, Error as ClientError,
    McpClient, McpClientTrait, McpService, SseTransport, StdioTransport, Transport,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tower::{timeout::Timeout, Service};

use super::{apply_tool_filters, BarkTool, BarkToolCall, BarkToolCallResponse};

pub trait McpServiceClient {
    fn call_mcp(
        &mut self,
        tool_name: &str,
        arguments: Value,
    ) -> core::result::Result<mcp_core::protocol::CallToolResult, mcp_client::Error>;

    fn list_mcp_tools(
        &self,
        next_cursor: Option<String>,
    ) -> Result<ListToolsResult, mcp_client::Error>;
}

impl<S> McpServiceClient for McpClient<S>
where
    S: Service<JsonRpcMessage, Response = JsonRpcMessage> + Clone + Send + Sync + 'static,
    S::Error: Into<mcp_client::Error>,
    S::Future: Send,
{
    fn call_mcp(
        &mut self,
        tool_name: &str,
        arguments: Value,
    ) -> core::result::Result<mcp_core::protocol::CallToolResult, mcp_client::Error> {
        block_on(self.call_tool(tool_name, arguments))
    }

    fn list_mcp_tools(
        &self,
        next_cursor: Option<String>,
    ) -> Result<ListToolsResult, mcp_client::Error> {
        block_on(self.list_tools(next_cursor))
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
    config: &McpServiceConfig,
) -> Result<Box<dyn McpServiceClient>, ClientError> {
    // 1) Create the transport
    let transport = StdioTransport::new(
        config.command.clone(),
        config.args.clone(),
        config.env.clone(),
    );

    // 2) Start the transport to get a handle
    let transport_handle = transport.start().await?;

    // 3) Create the service with timeout middleware
    let service = McpService::with_timeout(
        transport_handle,
        Duration::from_secs_f32(config.timeout_seconds),
    );

    // 4) Create the client with the middleware-wrapped service
    let mut client = McpClient::new(service);

    // Initialize
    let server_info = client
        .initialize(
            ClientInfo {
                name: "test-client".into(),
                version: "1.0.0".into(),
            },
            ClientCapabilities::default(),
        )
        .await?;

    Ok(Box::new(client))
}

async fn initialize_sse_mcp_service(host: &str) -> Result<Box<dyn McpServiceClient>, ClientError> {
    // 1) Create the transport
    let transport = SseTransport::new(host.to_string(), HashMap::new());

    // 2) Start the transport to get a handle
    let transport_handle = transport.start().await?;

    // 3) Create the service with timeout middleware
    let service = McpService::with_timeout(
        transport_handle,
        Duration::from_secs_f32(default_timeout_seconds()),
    );

    // 4) Create the client with the middleware-wrapped service
    let mut client = McpClient::new(service);

    // Initialize
    let server_info = client
        .initialize(
            ClientInfo {
                name: "test-client".into(),
                version: "1.0.0".into(),
            },
            ClientCapabilities::default(),
        )
        .await?;

    // Check if the server supports the SSE transport
    Ok(Box::new(client))
}

pub async fn initialize_mcp_service_map(
    config: &HashMap<String, McpServiceConfig>,
) -> HashMap<String, Arc<Mutex<Box<dyn McpServiceClient>>>> {
    let mut mcp_services = HashMap::new();
    for (name, service_config) in config.iter() {
        match initialize_stdio_mcp_service(service_config).await {
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
        match initialize_sse_mcp_service(host).await {
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

pub fn initialize_mcp_tool_map(
    clients: &HashMap<String, Arc<Mutex<Box<dyn McpServiceClient>>>>,
    filters: &HashMap<String, Vec<String>>,
) -> HashMap<String, BarkTool> {
    let mut mcp_tools = HashMap::new();
    for (name, client) in clients.iter() {
        let client = client.lock().unwrap();
        let tools = client.list_mcp_tools(None);
        match tools {
            Ok(tool_list) => {
                for tool in tool_list.tools.iter() {
                    let tool_name = tool.name.clone();
                    let tool_name = format!("{name}__{tool_name}");
                    if !apply_tool_filters(filters.get(name).unwrap_or(&Vec::new()), &tool_name) {
                        continue;
                    }
                    let tool_description = tool.description.clone();
                    let tool_parameters = tool.input_schema.clone();
                    mcp_tools.insert(
                        tool_name.clone(),
                        BarkTool {
                            name: tool_name,
                            description: tool_description,
                            parameters: tool_parameters,
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
            name: tool.name,
            description: tool.description,
            parameters: tool.input_schema,
        }
    }
}

impl From<BarkTool> for Tool {
    fn from(tool: BarkTool) -> Self {
        Self {
            name: tool.name,
            description: tool.description,
            input_schema: tool.parameters,
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
            match top {
                Content::Text(text) => Ok(BarkToolCallResponse {
                    id: call.id.clone(),
                    function_name: call.function_name.clone(),
                    arguments: call.arguments.clone(),
                    result: Some(text.text),
                }),
                _ => Err(format!("Unsupported tool response type")),
            }
        }
    }
}
