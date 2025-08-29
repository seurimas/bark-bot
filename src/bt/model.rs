use anyhow::anyhow;

use ollama_rs::{generation::embeddings::request::GenerateEmbeddingsRequest, Ollama};

use openai_api_rs::v1::embedding::EmbeddingResponse;
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

fn default_stripping() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BarkModelConfig<TC: ToolCaller = McpAndTree> {
    #[serde(default)]
    pub openai_models: HashMap<String, AiModelConfig>,
    #[serde(default)]
    pub ollama_models: HashMap<String, AiModelConfig>,
    #[serde(flatten)]
    pub tools: TC::Config,
    pub embedding_model: (String, String, Option<String>),
    #[serde(default = "default_stripping")]
    pub strip_thoughts_in_chat: bool,
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

#[derive(Debug, Clone)]
enum EmbeddingClientModel {
    OpenAI(OpenAI, String),
    Ollama(Ollama, String),
}

impl EmbeddingClientModel {
    pub async fn embeddings_create(&self, text: String) -> Result<(Vec<f32>, usize), String> {
        match self {
            EmbeddingClientModel::OpenAI(client, model_name) => client
                .embeddings_create(model_name, vec![text])
                .await
                .and_then(|mut response: EmbeddingResponse| {
                    let usage = response.usage.total_tokens;
                    Ok((
                        response
                            .data
                            .pop()
                            .ok_or_else(|| "No embeddings returned from OpenAI".to_string())?
                            .embedding,
                        usage as usize,
                    ))
                }),
            EmbeddingClientModel::Ollama(client, model_name) => client
                .generate_embeddings(GenerateEmbeddingsRequest::new(
                    model_name.clone(),
                    text.into(),
                ))
                .await
                .map_err(|e| format!("Error generating embeddings: {:?}", e))
                .and_then(|mut response| {
                    Ok((
                        response
                            .embeddings
                            .pop()
                            .ok_or_else(|| "No embeddings returned from Ollama".to_string())?,
                        0,
                    ))
                }),
        }
    }
}

#[derive(Clone)]
pub struct BarkModel<TC: ToolCaller = McpAndTree> {
    pub tree_root: String,
    openai_clients: HashMap<String, (String, OpenAI, Option<f32>)>,
    ollama_clients: HashMap<String, (String, Ollama, Option<f32>)>,
    tools: TC,
    embedding_client: EmbeddingClientModel,
    pub strip_thoughts_in_chat: bool,
}

impl<TC: ToolCaller> std::fmt::Debug for BarkModel<TC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BarkModel")
            .field("openai_clients", &self.openai_clients)
            .field("ollama_clients", &self.ollama_clients)
            .field("tools", &self.tools.debug())
            .field("embedding_client", &self.embedding_client)
            .finish()
    }
}

impl<TC: ToolCaller> BarkModel<TC> {
    pub async fn new(config: BarkModelConfig<TC>, tree_root: String) -> Self {
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

        let embedding_client = if config.embedding_model.2.is_some() {
            EmbeddingClientModel::OpenAI(
                OpenAI::new(
                    &config.embedding_model.2.unwrap(),
                    &config.embedding_model.1.clone(),
                ),
                config.embedding_model.0.clone(),
            )
        } else {
            EmbeddingClientModel::Ollama(
                Ollama::try_new(&config.embedding_model.1).unwrap(),
                config.embedding_model.0.clone(),
            )
        };
        let tools = TC::from_config(&config.tools).await;

        Self {
            tree_root,
            openai_clients,
            ollama_clients,
            tools,
            embedding_client,
            strip_thoughts_in_chat: config.strip_thoughts_in_chat,
        }
    }

    pub fn get_tools(&self, filters: &Vec<String>) -> Vec<BarkTool> {
        if filters.iter().any(|filter| filter.eq("debug")) {
            return vec![BarkTool::debug_tool()];
        } else {
            self.tools.get_tools(filters)
        }
    }

    pub async fn call_tool(
        self,
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
        self.tools.call_tool(tool_call, messages).await
    }

    pub async fn chat_completion_create(
        self,
        model: Option<String>,
        mut chat: BarkChat,
        tools: Vec<BarkTool>,
    ) -> Result<BarkResponse, String> {
        let model = model.unwrap_or("default".to_string());
        if let Some((model_name, client, temperature)) = self.openai_clients.get(&model) {
            chat.model = model_name.clone();
            chat.temperature = *temperature;
            crate::clients::openai_get_bark_response(client, chat, &tools).await
        } else if let Some((model_name, client, temperature)) = self.ollama_clients.get(&model) {
            chat.model = model_name.clone();
            chat.temperature = *temperature;
            crate::clients::ollama_get_bark_response(client, chat, &tools).await
        } else {
            Err(format!("Model {} not found", model))
        }
    }

    pub async fn get_embedding(
        self,
        text: String,
        mut gas: Option<i32>,
    ) -> Result<(Vec<f32>, Option<i32>), String> {
        self.embedding_client
            .embeddings_create(text.clone())
            .await
            .and_then(|(embedding, usage)| {
                if let Some(gas) = &mut gas {
                    *gas -= usage as i32;
                }
                Ok((embedding, gas))
            })
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
            // println!("Created tables");
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
        // println!("Row ID: {}", row_id);
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
                // eprintln!("Error: {:?}", err);
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
