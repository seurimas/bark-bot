use openai_api_rust::{chat::ChatApi, embeddings::EmbeddingsApi, Auth};
use rusqlite::{ffi::sqlite3_auto_extension, Connection};
use sqlite_vec::sqlite3_vec_init;
use zerocopy::AsBytes;

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct BarkModel {
    pub client: OpenAI,
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

    pub fn get_embedding(&self, text: &String) -> Result<Vec<f32>, openai_api_rust::Error> {
        self.client
            .embeddings_create(&openai_api_rust::embeddings::EmbeddingsBody {
                user: None,
                model: "BAAI/bge-small-en-v1.5".to_string(),
                input: vec![text.clone()],
            })
            .and_then(|response| {
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

    pub fn pull_best_match(
        &self,
        path: String,
        embedding: Vec<f32>,
    ) -> Result<String, rusqlite::Error> {
        let db = Connection::open(path)?;
        let mut stmt = db.prepare(
            "select rowid, distance from embeddings where embedding MATCH ?1 order by distance limit 1",
        )?;
        let result = stmt
            .query_map([embedding.as_bytes()], |r| Ok(r.get::<_, i64>(0).unwrap()))?
            .collect::<Result<Vec<i64>, _>>();
        match result {
            Ok(result) => {
                if result.is_empty() {
                    return Err(rusqlite::Error::QueryReturnedNoRows);
                }
                let rowid = result[0];
                let mut stmt = db.prepare("select value from texts where rowid = ?")?;
                let result = stmt
                    .query_map([rowid], |r| Ok(r.get(0).unwrap()))?
                    .collect::<Result<Vec<String>, _>>()?;
                Ok(result[0].clone())
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                Err(err)
            }
        }
    }
}
