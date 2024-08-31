pub use crate::bt::values::{MessageValue, PromptValue, TextValue, VariableId};
use crate::bt::BarkDef;
pub use crate::bt::BarkNode;
pub use crate::bt::{BarkController, BarkFunction, BarkModel};
pub use behavior_bark::unpowered::*;

use openai_api_rust::chat::ChatBody;
use openai_api_rust::Role;
pub use openai_api_rust::{Message, OpenAI};
pub use std::collections::HashMap;

pub use serde::{Deserialize, Serialize};

pub fn user(s: &impl ToString) -> Message {
    Message {
        role: Role::User,
        content: s.to_string(),
    }
}

pub fn system(s: &impl ToString) -> Message {
    Message {
        role: Role::System,
        content: s.to_string(),
    }
}

pub fn chat(prompt: Vec<Message>) -> ChatBody {
    let mut combined: Vec<Message> = vec![];
    for message in prompt {
        if let Some(top) = combined.last_mut() {
            if matches!(top.role, Role::User) == matches!(message.role, Role::User) {
                top.content.push_str(&message.content);
                continue;
            }
        }
        combined.push(message);
    }
    ChatBody {
        frequency_penalty: None,
        logit_bias: None,
        max_tokens: Some(4096),
        messages: combined,
        model: "dolphin-2.1-mistral-7b.Q4_K_M.gguf".to_string(),
        n: None,
        presence_penalty: None,
        stop: None,
        stream: None,
        temperature: None,
        top_p: None,
        user: None,
    }
}

pub fn score(embed_a: &[f32], embed_b: &[f32]) -> f32 {
    let mut sum = 0.0;
    for (a, b) in embed_a.iter().zip(embed_b.iter()) {
        sum += (a - b).powi(2);
    }
    println!("{}", sum);
    sum
}

pub fn read_tree(tree_path: &str) -> BarkDef {
    let tree = std::fs::read_to_string(tree_path).expect("Failed to read tree file");
    let tree: crate::bt::BarkDef = serde_json::from_str(&tree).expect("Failed to parse tree file");
    tree
}

pub fn unpowered_prompt(
    prompt: Vec<Message>,
    model: &BarkModel,
) -> (String, UnpoweredFunctionState) {
    match model.chat_completion_create(&chat(prompt)) {
        Ok(mut response) => {
            if response.choices.is_empty() {
                eprintln!("Prompt Error (empty): {:?}", response);
                return ("".to_string(), UnpoweredFunctionState::Failed);
            } else if response.choices[0].message.is_none() {
                eprintln!("Prompt Error (empty message): {:?}", response);
                return ("".to_string(), UnpoweredFunctionState::Failed);
            } else if response.choices.len() > 1 {
                eprintln!("Prompt Warning (multiple choices): {:?}", response);
            }
            (
                response.choices.pop().unwrap().message.unwrap().content,
                UnpoweredFunctionState::Complete,
            )
        }
        Err(e) => {
            eprintln!("Prompt Error: {:?}", e);
            ("".to_string(), UnpoweredFunctionState::Failed)
        }
    }
}
