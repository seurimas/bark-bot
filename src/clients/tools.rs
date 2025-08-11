use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::prelude::*;

pub fn apply_tool_filters(filters: &Vec<String>, tool_name: &String) -> bool {
    for filter in filters {
        if filter.starts_with("!") {
            if tool_name.contains(&filter[1..]) {
                return false;
            }
        } else if filter.starts_with("=") {
            if tool_name.eq(&filter[1..]) {
                return true;
            }
        } else if filter.starts_with("@") {
            if tool_name.starts_with(&filter[1..]) {
                return true;
            }
        } else if filter.starts_with("*") {
            if tool_name.contains(&filter[1..]) {
                return true;
            }
        }
    }
    filters.is_empty()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TreeServiceConfig {
    pub path: String,
    pub description: String,
    pub parameters: Value,
}

pub trait ToolCaller: Clone {
    type Config: std::fmt::Debug + Clone + serde::de::DeserializeOwned + serde::Serialize;

    fn from_config(config: &Self::Config) -> impl std::future::Future<Output = Self> + Send;

    fn get_tools(&self, filters: &Vec<String>) -> Vec<BarkTool>;

    fn call_tool(
        self,
        tool_call: &BarkToolCall,
        messages: &Vec<BarkMessage>,
    ) -> impl std::future::Future<Output = Result<BarkToolCallResponse, String>> + Send;

    fn debug(&self) -> String;
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct McpAndTreeConfig {
    #[serde(default)]
    pub mcp_services: HashMap<String, McpServiceConfig>,
    #[serde(default)]
    pub mcp_sse_hosts: HashMap<String, String>,
    #[serde(default)]
    pub tree_services: HashMap<String, TreeServiceConfig>,
}

#[derive(Clone)]
pub struct McpAndTree {
    mcp_services: HashMap<String, RunningServiceClient>,
    tree_services: HashMap<String, BarkDef>,
    tools_map: HashMap<String, BarkTool>,
}

impl ToolCaller for McpAndTree {
    type Config = McpAndTreeConfig;

    async fn from_config(config: &Self::Config) -> Self {
        let mut mcp_services = initialize_mcp_service_map(&config.mcp_services).await;
        mcp_services.extend(initialize_sse_mcp_service_map(&config.mcp_sse_hosts).await);
        let service_filters = config
            .mcp_services
            .iter()
            .map(|(name, config)| {
                let filters = config.tool_filters.clone();
                (name.clone(), filters)
            })
            .collect::<HashMap<String, Vec<String>>>();
        let mut tools_map = initialize_mcp_tool_map(&mcp_services, &service_filters).await;

        // let tree_services = config
        //     .tree_services
        //     .iter()
        //     .map(|(name, config)| {
        //         let tree = read_tree(&tree_root, &config.path);
        //         (name.clone(), tree)
        //     })
        //     .collect::<HashMap<String, BarkDef>>();
        // for (name, _tree) in &tree_services {
        //     tools_map.insert(
        //         format!("tool__{}", name),
        //         BarkTool {
        //             name: name.clone(),
        //             description: config.tree_services[name].description.clone(),
        //             parameters: config.tree_services[name].parameters.clone(),
        //         },
        //     );
        // }
        Self {
            mcp_services,
            // tree_services,
            tree_services: HashMap::new(), // Disable tree services for now
            tools_map,
        }
    }

    fn get_tools(&self, filters: &Vec<String>) -> Vec<BarkTool> {
        return self
            .tools_map
            .iter()
            .filter_map(|entry| {
                if apply_tool_filters(filters, entry.0) {
                    Some(entry.1.clone())
                } else {
                    None
                }
            })
            .collect();
    }

    async fn call_tool(
        self,
        tool_call: &BarkToolCall,
        messages: &Vec<BarkMessage>,
    ) -> Result<BarkToolCallResponse, String> {
        let Some((prefix, function_name)) = tool_call.function_name.split_once("__") else {
            return Err(format!(
                "Invalid function name format: {}",
                tool_call.function_name
            ));
        };
        // XXX: Tree services are disabled for now
        // if prefix == "tool" && self.tree_services.contains_key(function_name) {
        //     let tree_service = self.tree_services.get(function_name).unwrap();
        //     let mut tree_service = tree_service.create_tree();
        //     let mut controller = crate::bt::BarkController::new();
        //     if let Some(arguments) = tool_call
        //         .arguments
        //         .clone()
        //         .and_then(|args| serde_json::from_str::<HashMap<String, String>>(&args).ok())
        //     {
        //         for (key, value) in arguments {
        //             controller
        //                 .text_variables
        //                 .insert(VariableId::PreLoaded(key), value);
        //         }
        //     }
        //     controller
        //         .prompts
        //         .insert(VariableId::LastOutput, messages.clone());
        //     tree_service.resume_with(&self, &mut controller, &mut None, &mut None);
        //     let response = BarkToolCallResponse {
        //         id: tool_call.id.clone(),
        //         result: controller
        //             .text_variables
        //             .get(&VariableId::LastOutput)
        //             .cloned(),
        //         arguments: tool_call.arguments.clone(),
        //         function_name: tool_call.function_name.clone(),
        //     };
        //     return Ok(response);
        // } else
        if let Some(mcp_service) = self.mcp_services.get(prefix) {
            mcp_service
                .call_mcp(
                    function_name,
                    tool_call
                        .arguments
                        .clone()
                        .and_then(|args| serde_json::from_str::<Value>(&args).ok())
                        .unwrap_or(Value::Object(serde_json::Map::new())),
                )
                .await
                .unwrap_or(Err(anyhow!("Failed to call MCP service {}", function_name)))
                .map_err(|e| e.to_string())
                .and_then(|response| BarkToolCallResponse::try_parse(tool_call, response))
        } else {
            Err(format!("Tool {} not found", tool_call.function_name))
        }
    }

    fn debug(&self) -> String {
        format!(
            "Tools Map: {:?}",
            self.tools_map.keys().cloned().collect::<Vec<String>>(),
        )
    }
}
