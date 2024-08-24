use openai_api_rust::{chat::ChatApi, embeddings::EmbeddingsApi, Auth};

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct BarkModel {
    pub client: OpenAI,
}

impl BarkModel {
    pub fn new() -> Self {
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

    pub fn get_embedding(&self, text: String) -> Result<Vec<f64>, openai_api_rust::Error> {
        self.client
            .embeddings_create(&openai_api_rust::embeddings::EmbeddingsBody {
                user: None,
                model: "BAAI/bge-small-en-v1.5".to_string(),
                input: vec![text],
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
}
