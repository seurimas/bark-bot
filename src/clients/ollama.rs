use ollama_rs::generation::options::GenerationOptions;

use super::{BarkChat, BarkResponse, BarkRole};

impl From<ollama_rs::generation::chat::ChatMessageResponse> for BarkResponse {
    fn from(response: ollama_rs::generation::chat::ChatMessageResponse) -> Self {
        Self {
            choices: vec![super::Choice {
                index: 0,
                value: response.message.content,
            }],
            usage: response
                .final_data
                .map(|data| (data.eval_count + data.prompt_eval_count) as u32),
        }
    }
}

impl From<BarkChat> for ollama_rs::generation::chat::request::ChatMessageRequest {
    fn from(chat: BarkChat) -> Self {
        let mut combined: Vec<ollama_rs::generation::chat::ChatMessage> = vec![];
        let mut combined_message = ollama_rs::generation::chat::ChatMessage {
            role: ollama_rs::generation::chat::MessageRole::User,
            content: "".to_string(),
            tool_calls: vec![],
            images: None,
        };
        combined.push(ollama_rs::generation::chat::ChatMessage {
                role: ollama_rs::generation::chat::MessageRole::System,
                content: "Respond helpfully and concisely to queries. For very complicated queries, think it through first. Otherwise, just answer.".to_string(),
                tool_calls: vec![],
                images: None,
            });
        for message in chat.messages {
            if let Some(top) = combined.last_mut() {
                if matches!(top.role, ollama_rs::generation::chat::MessageRole::User)
                    == matches!(message.role, BarkRole::User)
                {
                    top.content.push_str(&message.content);
                    continue;
                } else if matches!(
                    top.role,
                    ollama_rs::generation::chat::MessageRole::Assistant
                ) == matches!(message.role, BarkRole::Assistant)
                {
                    top.content.push_str(&message.content);
                    continue;
                } else if matches!(top.role, ollama_rs::generation::chat::MessageRole::System)
                    == matches!(message.role, BarkRole::System)
                {
                    top.content.push_str(&message.content);
                    continue;
                }
            }
            combined_message.role = match message.role {
                BarkRole::User => ollama_rs::generation::chat::MessageRole::User,
                BarkRole::Assistant => ollama_rs::generation::chat::MessageRole::Assistant,
                BarkRole::System => ollama_rs::generation::chat::MessageRole::System,
            };
            combined.push(combined_message);
            combined_message = ollama_rs::generation::chat::ChatMessage {
                role: ollama_rs::generation::chat::MessageRole::User,
                content: "".to_string(),
                tool_calls: vec![],
                images: None,
            };
        }
        if let Some(temperature) = chat.temperature {
            ollama_rs::generation::chat::request::ChatMessageRequest::new(chat.model, combined)
                .options(GenerationOptions::default().temperature(temperature))
        } else {
            ollama_rs::generation::chat::request::ChatMessageRequest::new(chat.model, combined)
        }
    }
}
