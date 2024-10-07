use openai_api_rust::{chat::ChatApi, embeddings::EmbeddingsApi, Auth};

use rusqlite::{ffi::sqlite3_auto_extension, Connection};
use sqlite_vec::sqlite3_vec_init;
use zerocopy::AsBytes;

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct BarkModel {
    client: OpenAI,
}

impl BarkModel {
    pub fn new() -> Self {
        unsafe {
            sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
        }

        let auth = Auth::from_env().unwrap();
        let client = OpenAI::new(auth, &std::env::var("OPENAI_URL").unwrap());

        Self { client }
    }

    pub fn chat_completion_create(
        &self,
        chat: &openai_api_rust::chat::ChatBody,
    ) -> Result<openai_api_rust::completions::Completion, openai_api_rust::Error> {
        self.client.chat_completion_create(chat)
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
        self.client
            .embeddings_create(&openai_api_rust::embeddings::EmbeddingsBody {
                user: None,
                model: "BAAI/bge-small-en-v1.5".to_string(),
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
            return line;
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
