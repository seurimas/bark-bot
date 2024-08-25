pub use crate::bt::values::{MessageValue, PromptValue, TextValue, VariableId};
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
        max_tokens: None,
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
