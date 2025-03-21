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

use super::{BarkTool, BarkToolCall, BarkToolCallResponse};

pub type McpServiceClient = McpClient<Timeout<McpService<StdioTransportHandle>>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServiceConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub timeout_seconds: f32,
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
    println!("Connected to server: {server_info:?}\n");

    // List tools
    let tools = client.list_tools(None).await?;
    println!("Available tools: {tools:?}\n");

    // List resources
    let resources = client.list_resources(None).await?;
    println!("Available resources: {resources:?}\n");

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
