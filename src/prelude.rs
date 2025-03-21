pub use crate::bt::values::{MessageValue, PromptValue, TextMatcher, TextValue, VariableId};
pub use crate::bt::BarkDef;
pub use crate::bt::BarkNode;
pub use crate::bt::{BarkController, BarkFunction, BarkModel, BarkState};
pub use behavior_bark::powered::*;

pub use behavior_bark::check_gas;

pub use crate::clients::*;
use once_cell::sync::OnceCell;
pub use std::collections::HashMap;

pub use serde::{Deserialize, Serialize};

pub fn user(s: &impl ToString) -> BarkMessage {
    BarkMessage {
        role: BarkRole::User,
        content: s.to_string(),
    }
}

pub fn system(s: &impl ToString) -> BarkMessage {
    BarkMessage {
        role: BarkRole::System,
        content: s.to_string(),
    }
}

pub fn score(embed_a: &[f32], embed_b: &[f32]) -> f32 {
    let mut sum = 0.0;
    for (a, b) in embed_a.iter().zip(embed_b.iter()) {
        sum += (a - b).powi(2);
    }
    sum
}

pub static TREE_ROOT: OnceCell<String> = OnceCell::new();

pub fn read_tree(tree_path: &str) -> BarkDef {
    let root = std::path::Path::new(TREE_ROOT.get().expect("TREE_ROOT not set"));
    let tree = std::fs::read_to_string(std::path::Path::join(root, tree_path))
        .expect("Failed to read tree file");
    let tree: crate::bt::BarkDef = if tree_path.ends_with("json") {
        serde_json::from_str(&tree).expect("Failed to parse JSON tree file")
    } else {
        ron::from_str(&tree).expect("Failed to parse RON tree file")
    };
    tree
}

pub fn powered_prompt(
    preferred_model: Option<&String>,
    prompt: Vec<BarkMessage>,
    model: &BarkModel,
    gas: &mut Option<i32>,
    tools: Vec<&String>,
) -> (String, BarkState) {
    match model.chat_completion_create(preferred_model, prompt.into()) {
        Ok(mut response) => {
            if let Some(gas) = gas {
                *gas = *gas - response.usage.unwrap_or(1000) as i32;
            }
            if response.choices.is_empty() {
                eprintln!("Prompt Error (empty): {:?}", response);
                return ("".to_string(), BarkState::Failed);
            } else if response.choices[0].value.is_empty() {
                eprintln!("Prompt Error (empty message): {:?}", response);
                return ("".to_string(), BarkState::Failed);
            } else if response.choices.len() > 1 {
                eprintln!("Prompt Warning (multiple choices): {:?}", response);
            }
            (response.choices.pop().unwrap().value, BarkState::Complete)
        }
        Err(e) => {
            eprintln!("Prompt Error: {:?}", e);
            ("".to_string(), BarkState::Failed)
        }
    }
}
