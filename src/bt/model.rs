use openai_api_rust::{chat::ChatApi, embeddings::EmbeddingsApi, Auth};

use rusqlite::{ffi::sqlite3_auto_extension, Connection};
use sqlite_vec::sqlite3_vec_init;
use zerocopy::AsBytes;

use crate::prelude::*;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AiModelConfig {
    pub model_name: String,
    pub api_key: String,
    pub url: String,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BarkModelConfig {
    pub models: HashMap<String, AiModelConfig>,
    pub embedding_model: (String, String, String),
}

impl BarkModelConfig {
    pub fn get_from_env() -> Self {
        if let (Ok(auth), Some(url)) = (Auth::from_env(), &std::env::var("OPENAI_URL").ok()) {
            let mut models = HashMap::new();
            let model = std::env::var("MODEL_NAME").unwrap_or("mistral-nemo".to_string());
            models.insert(
                "default".to_string(),
                AiModelConfig {
                    model_name: model.clone(),
                    api_key: auth.api_key.clone(),
                    url: url.clone(),
                    temperature: None,
                },
            );
            let embedding_model = (
                std::env::var("EMBEDDING_MODEL_NAME")
                    .unwrap_or("BAAI/bge-small-en-v1.5".to_string()),
                auth.api_key.clone(),
                url.clone(),
            );

            Self {
                models,
                embedding_model,
            }
        } else {
            panic!("Failed to get OpenAI auth from environment");
        }
    }
}

#[derive(Debug, Clone)]
pub struct BarkModel {
    clients: HashMap<String, (String, OpenAI, Option<f32>)>,
    embedding_client: OpenAI,
    embedding_model: String,
}

impl BarkModel {
    pub fn new(config: BarkModelConfig) -> Self {
        unsafe {
            sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
        }

        let clients = config
            .models
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
                            OpenAI::new(Auth::new(api_key), url),
                            temperature.clone(),
                        ),
                    )
                },
            )
            .collect();
        let embedding_client = OpenAI::new(
            Auth::new(config.embedding_model.2.as_str()),
            config.embedding_model.1.as_str(),
        );
        let embedding_model = config.embedding_model.0.clone();

        Self {
            clients,
            embedding_client,
            embedding_model,
        }
    }

    pub fn chat_completion_create(
        &self,
        model: Option<&String>,
        mut chat: openai_api_rust::chat::ChatBody,
    ) -> Result<openai_api_rust::completions::Completion, openai_api_rust::Error> {
        let model = model.unwrap_or(&"default".to_string()).clone();
        let (model_name, client, temperature) = self.clients.get(&model).unwrap();
        chat.model = model_name.clone();
        chat.temperature = *temperature;
        client.chat_completion_create(&chat)
    }

    pub fn search(&self, query: &str) {
        let response = ureq::get(
            format!(
                "{}?key={}&cx={}&q={}",
                "https://www.googleapis.com/customsearch/v1",
                std::env::var("GOOGLE_API_KEY").unwrap(),
                std::env::var("GOOGLE_CX").unwrap(),
                query
            )
            .as_str(),
        )
        .call();
        println!("{:?}", response);
    }

    pub fn get_embedding(
        &self,
        text: &String,
        gas: &mut Option<i32>,
    ) -> Result<Vec<f32>, openai_api_rust::Error> {
        self.embedding_client
            .embeddings_create(&openai_api_rust::embeddings::EmbeddingsBody {
                user: None,
                model: self.embedding_model.clone(),
                input: vec![text.clone()],
            })
            .and_then(|response| {
                response.usage.total_tokens.map(|tokens| {
                    if let Some(gas) = gas {
                        *gas = *gas - tokens as i32;
                    }
                });
                response
                    .data
                    .ok_or(openai_api_rust::Error::ApiError("No data".to_string()))
            })
            .and_then(|data| {
                data[0]
                    .embedding
                    .clone()
                    .ok_or(openai_api_rust::Error::ApiError("No embedding".to_string()))
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
