use mcp_core::types::{CallToolRequest, CallToolResponse, Tool, ToolResponseContent};
use mcp_spec::{protocol::CallToolResult, Content};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use mcp_client::{
    transport::stdio::StdioTransportHandle, ClientCapabilities, ClientInfo, Error as ClientError,
    McpClient, McpClientTrait, McpService, StdioTransport, Transport,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tower::timeout::Timeout;

use super::{apply_tool_filters, BarkTool, BarkToolCall, BarkToolCallResponse};

pub type McpServiceClient = McpClient<Timeout<McpService<StdioTransportHandle>>>;

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

pub async fn initialize_mcp_service(
    config: &McpServiceConfig,
) -> Result<McpServiceClient, ClientError> {
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

    Ok(client)
}

pub async fn initialize_mcp_service_map(
    config: &HashMap<String, McpServiceConfig>,
) -> HashMap<String, Arc<Mutex<McpServiceClient>>> {
    let mut mcp_services = HashMap::new();
    for (name, service_config) in config.iter() {
        match initialize_mcp_service(service_config).await {
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
    clients: &HashMap<String, Arc<Mutex<McpServiceClient>>>,
    filters: &HashMap<String, Vec<String>>,
) -> HashMap<String, BarkTool> {
    let mut mcp_tools = HashMap::new();
    for (name, client) in clients.iter() {
        let client = client.lock().unwrap();
        let tools = client.list_tools(None).await;
        match tools {
            Ok(tool_list) => {
                for tool in tool_list.tools.iter() {
                    let tool_name = tool.name.clone();
                    let tool_name = format!("{name}__{tool_name}");
                    if !apply_tool_filters(filters, &tool_name) {
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
            description: tool.description.unwrap_or_default(),
            parameters: tool.input_schema,
        }
    }
}

impl From<BarkTool> for Tool {
    fn from(tool: BarkTool) -> Self {
        Self {
            name: tool.name,
            description: Some(tool.description),
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
