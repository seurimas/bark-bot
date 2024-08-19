pub use crate::bt::BarkNode;
pub use crate::bt::{BarkController, BarkFunction, BarkModel};
pub use crate::bt::{PromptValue, VariableId};
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
    ChatBody {
        frequency_penalty: None,
        logit_bias: None,
        max_tokens: None,
        messages: prompt,
        model: "".to_string(),
        n: None,
        presence_penalty: None,
        stop: None,
        stream: None,
        temperature: None,
        top_p: None,
        user: None,
    }
}
