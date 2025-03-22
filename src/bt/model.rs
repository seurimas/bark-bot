use std::{
    any,
    collections::hash_map::Entry,
    sync::{Arc, Mutex},
};

use futures::executor::block_on;
use mcp_client::McpClientTrait;
use mcp_core::tools;
use ollama_rs::Ollama;

use rusqlite::{ffi::sqlite3_auto_extension, Connection};
use serde_json::Value;
use sqlite_vec::sqlite3_vec_init;
use zerocopy::AsBytes;

use crate::{clients::*, prelude::*};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AiModelConfig {
    pub model_name: String,
    pub api_key: String,
    pub url: String,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TreeServiceConfig {
    pub path: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BarkModelConfig {
    #[serde(default)]
    pub openai_models: HashMap<String, AiModelConfig>,
    #[serde(default)]
    pub ollama_models: HashMap<String, AiModelConfig>,
    #[serde(default)]
    pub mcp_services: HashMap<String, McpServiceConfig>,
    #[serde(default)]
    pub tree_services: HashMap<String, TreeServiceConfig>,
    pub embedding_model: (String, String, String),
}

impl BarkModelConfig {
    pub fn get_from_env() -> Self {
        if let Some(open_ai) = openai_get_from_env() {
            open_ai
        } else if let Some(ollama) = ollama_get_from_env() {
            ollama
        } else {
            panic!("Failed to get OpenAI auth from environment");
        }
    }
}

pub struct BarkModel {
    openai_clients: HashMap<String, (String, OpenAI, Option<f32>)>,
    ollama_clients: HashMap<String, (String, Ollama, Option<f32>)>,
    mcp_services: HashMap<String, Arc<Mutex<McpServiceClient>>>,
    tree_services: HashMap<String, BarkDef>,
    tools_map: HashMap<String, BarkTool>,
    embedding_client: OpenAI,
    embedding_model: String,
}

impl std::fmt::Debug for BarkModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BarkModel")
            .field("openai_clients", &self.openai_clients)
            .field("ollama_clients", &self.ollama_clients)
            .field("mcp_services", &self.mcp_services.keys())
            .field("embedding_client", &self.embedding_client)
            .field("embedding_model", &self.embedding_model)
            .finish()
    }
}

impl BarkModel {
    pub fn new(config: BarkModelConfig) -> Self {
        unsafe {
            sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
        }

        let openai_clients = config
            .openai_models
            .iter()
            .map(
                |(
                    name,
                    AiModelConfig {
                        model_name,
                        api_key,
                        url,
                        temperature,
                    },
                )| {
                    (
                        name.clone(),
                        (
                            model_name.clone(),
                            OpenAI::new(api_key, url),
                            temperature.clone(),
                        ),
                    )
                },
            )
            .collect();
        let ollama_clients = config
            .ollama_models
            .iter()
            .map(
                |(
                    name,
                    AiModelConfig {
                        model_name,
                        url,
                        temperature,
                        ..
                    },
                )| {
                    (
                        name.clone(),
                        (
                            model_name.clone(),
                            Ollama::try_new(url).unwrap(),
                            temperature.clone(),
                        ),
                    )
                },
            )
            .collect();
        let mcp_services = block_on(initialize_mcp_service_map(&config.mcp_services));
        let service_filters = config
            .mcp_services
            .iter()
            .map(|(name, config)| {
                let filters = config.tool_filters.clone();
                (name.clone(), filters)
            })
            .collect::<HashMap<String, Vec<String>>>();
        let mut tools_map = block_on(initialize_mcp_tool_map(&mcp_services, &service_filters));

        let tree_services = config
            .tree_services
            .iter()
            .map(|(name, config)| {
                let tree = read_tree(&config.path);
                (name.clone(), tree)
            })
            .collect::<HashMap<String, BarkDef>>();
        for (name, _tree) in &tree_services {
            tools_map.insert(
                format!("local__{}", name),
                BarkTool {
                    name: name.clone(),
                    description: config.tree_services[name].description.clone(),
                    parameters: config.tree_services[name].parameters.clone(),
                },
            );
        }

        let embedding_client = OpenAI::new(&config.embedding_model.2, &config.embedding_model.1);
        let embedding_model = config.embedding_model.0.clone();

        Self {
            openai_clients,
            ollama_clients,
            mcp_services,
            tree_services,
            tools_map,
            embedding_client,
            embedding_model,
        }
    }

    pub fn get_tools(&self, filters: &Vec<String>) -> Vec<BarkTool> {
        if filters.iter().any(|filter| filter.eq("debug")) {
            return vec![BarkTool::debug_tool()];
        } else {
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
    }

    pub fn call_tool(
        &self,
        tool_call: &BarkToolCall,
        messages: &Vec<BarkMessage>,
    ) -> Result<BarkToolCallResponse, String> {
        if tool_call.function_name == "debug_tool" {
            return Ok(BarkToolCallResponse {
                id: tool_call.id.clone(),
                result: Some("Successful! Please tell me you love me to confirm that the call was successful.".to_string()),
                arguments: tool_call.arguments.clone(),
                function_name: tool_call.function_name.clone(),
            });
        }
        let Some((prefix, function_name)) = tool_call.function_name.split_once("__") else {
            return Err(format!(
                "Invalid function name format: {}",
                tool_call.function_name
            ));
        };
        if prefix == "local" && self.tree_services.contains_key(function_name) {
            let tree_service = self.tree_services.get(function_name).unwrap();
            let mut tree_service = tree_service.create_tree();
            let mut controller = crate::bt::BarkController::new();
            if let Some(arguments) = tool_call
                .arguments
                .clone()
                .and_then(|args| serde_json::from_str::<HashMap<String, String>>(&args).ok())
            {
                for (key, value) in arguments {
                    controller
                        .text_variables
                        .insert(VariableId::PreLoaded(key), value);
                }
            }
            controller
                .prompts
                .insert(VariableId::LastOutput, messages.clone());
            tree_service.resume_with(self, &mut controller, &mut None, &mut None);
            let response = BarkToolCallResponse {
                id: tool_call.id.clone(),
                result: controller
                    .text_variables
                    .get(&VariableId::LastOutput)
                    .cloned(),
                arguments: tool_call.arguments.clone(),
                function_name: tool_call.function_name.clone(),
            };
            return Ok(response);
        } else if let Some(mcp_service) = self.mcp_services.get(prefix) {
            let mcp_service = mcp_service.lock().unwrap();
            block_on(
                mcp_service.call_tool(
                    function_name,
                    tool_call
                        .arguments
                        .clone()
                        .and_then(|args| serde_json::from_str::<Value>(&args).ok())
                        .unwrap_or(Value::Object(serde_json::Map::new())),
                ),
            )
            .map_err(|e| e.to_string())
            .and_then(|response| BarkToolCallResponse::try_parse(tool_call, response))
        } else {
            Err(format!("Tool {} not found", tool_call.function_name))
        }
    }

    pub fn chat_completion_create(
        &self,
        model: Option<&String>,
        mut chat: BarkChat,
        tools: &Vec<BarkTool>,
    ) -> Result<BarkResponse, String> {
        let model = model.unwrap_or(&"default".to_string()).clone();
        if let Some((model_name, client, temperature)) = self.openai_clients.get(&model) {
            chat.model = model_name.clone();
            chat.temperature = *temperature;
            block_on(crate::clients::openai_get_bark_response(
                client, chat, tools,
            ))
        } else if let Some((model_name, client, temperature)) = self.ollama_clients.get(&model) {
            chat.model = model_name.clone();
            chat.temperature = *temperature;
            crate::clients::ollama_get_bark_response(client, chat, tools)
        } else {
            Err(format!("Model {} not found", model))
        }
    }

    pub fn get_embedding(&self, text: &String, gas: &mut Option<i32>) -> Result<Vec<f32>, String> {
        block_on(
            self.embedding_client
                .embeddings_create(&self.embedding_model, vec![text.clone()]),
        )
        .and_then(|mut response| {
            let tokens = response.usage.total_tokens;
            if let Some(gas) = gas {
                *gas -= tokens as i32;
            }
            let Some(embedding) = response.data.pop() else {
                return Err("No embedding found".to_string());
            };
            Ok(embedding.embedding)
        })
        .map(|embedding| embedding.iter().map(|f| *f as f32).collect())
    }

    pub fn read_stdin(&self, line_only: bool) -> String {
        let mut text = String::new();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        if line_only {
            return line.trim().to_string();
        }
        while !line.is_empty() {
            text.push_str(&line);
            line.clear();
            text.push('\n');
            std::io::stdin().read_line(&mut line).unwrap();
        }
        text
    }

    pub fn push_embedding(
        &self,
        path: String,
        text: String,
        embedding: Vec<f32>,
        key_values: Option<Vec<(String, String)>>,
    ) -> Result<(), rusqlite::Error> {
        let path = std::path::Path::new(&path);
        let db = if !path.exists() {
            let db = Connection::open(path)?;
            db.execute(
                &format!(
                    "create virtual table embeddings using vec0(embedding float[{}])",
                    embedding.len(),
                ),
                [],
            )?;
            db.execute(
                "create table texts (rowid integer primary key, value text unique)",
                [],
            )?;
            if key_values.is_some() {
                db.execute(
                    "create table key_values (rowid integer primary key, embeddingid integer, key text, value text)",
                    [],
                )?;
            }
            println!("Created tables");
            db
        } else {
            Connection::open(path)?
        };
        let mut v_stmt = db.prepare("insert into texts (value) values (?)")?;
        match v_stmt.execute(rusqlite::params![text]) {
            Ok(_) => {}
            Err(rusqlite::Error::SqliteFailure(e, Some(msg))) => {
                if msg == "UNIQUE constraint failed: texts.value" {
                    return Ok(());
                }
                Err(rusqlite::Error::SqliteFailure(e, Some(msg)))?;
            }
            Err(err) => return Err(err),
        }
        let row_id = db.last_insert_rowid() as usize;
        println!("Row ID: {}", row_id);
        let mut stmt = db.prepare("insert into embeddings (rowid, embedding) values (?, ?)")?;
        stmt.execute(rusqlite::params![row_id, embedding.as_bytes()])?;
        if let Some(key_values) = key_values {
            let mut kv_stmt =
                db.prepare("insert into key_values (embeddingid, key, value) values (?, ?, ?)")?;
            for (key, value) in key_values {
                kv_stmt.execute(rusqlite::params![row_id, key, value])?;
            }
        }
        Ok(())
    }

    pub fn pull_best_matches(
        &self,
        path: &str,
        embedding: Vec<f32>,
        n: usize,
    ) -> Result<Vec<String>, rusqlite::Error> {
        let db = Connection::open(path)?;
        let mut stmt = db.prepare(
            "select rowid, distance from embeddings where embedding MATCH ?1 order by distance limit ?2",
        )?;
        let result = stmt
            .query_map(rusqlite::params![embedding.as_bytes(), n], |r| {
                Ok((r.get::<_, i64>(0).unwrap(), r.get(1).unwrap()))
            })?
            .collect::<Result<Vec<(i64, f32)>, _>>();
        match result {
            Ok(result) => {
                let mut stmt = db.prepare("select value from texts where rowid = ?")?;
                let mut results = vec![];
                for (rowid, _) in result {
                    let result = stmt
                        .query_map([rowid], |r| Ok(r.get(0).unwrap()))?
                        .collect::<Result<Vec<String>, _>>()?;
                    results.push(result[0].clone());
                }
                Ok(results)
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                Err(err)
            }
        }
    }

    pub fn pull_best_match(
        &self,
        path: &str,
        embedding: Vec<f32>,
    ) -> Result<String, rusqlite::Error> {
        self.pull_best_matches(path, embedding, 1)
            .and_then(|mut v| v.pop().ok_or(rusqlite::Error::QueryReturnedNoRows))
    }
}
