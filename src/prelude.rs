pub use crate::bt::values::{MessageValue, PromptValue, TextMatcher, TextValue, VariableId};
pub use crate::bt::BarkDef;
pub use crate::bt::BarkNode;
pub use crate::bt::{BarkController, BarkFunction, BarkModel, BarkState};
pub use behavior_bark::powered::*;

pub use behavior_bark::check_gas;

pub use crate::clients::*;
use once_cell::sync::OnceCell;
pub use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub use serde::{Deserialize, Serialize};

pub fn user(s: &impl ToString) -> BarkMessage {
    BarkMessage {
        role: BarkRole::User,
        content: BarkContent::Text(s.to_string()),
    }
}

pub fn system(s: &impl ToString) -> BarkMessage {
    BarkMessage {
        role: BarkRole::System,
        content: BarkContent::Text(s.to_string()),
    }
}

pub fn score(embed_a: &[f32], embed_b: &[f32]) -> f32 {
    let mut sum = 0.0;
    for (a, b) in embed_a.iter().zip(embed_b.iter()) {
        sum += (a - b).powi(2);
    }
    sum
}

pub fn read_tree(root: impl AsRef<Path>, tree_path: &str) -> BarkDef {
    let root = root.as_ref();
    let path = std::path::Path::join(root, tree_path);
    let tree = std::fs::read_to_string(&path)
        .expect(format!("Failed to read tree file: {:?}", path).as_str());
    let tree: crate::bt::BarkDef = if tree_path.ends_with("json") {
        serde_json::from_str(&tree).expect("Failed to parse JSON tree file")
    } else if tree_path.ends_with("ron") {
        ron::from_str(&tree).expect("Failed to parse RON tree file")
    } else {
        panic!("Unsupported tree file format: {:?}", tree_path)
    };
    tree
}

pub fn powered_prompt(
    preferred_model: Option<&String>,
    prompt: Vec<BarkMessage>,
    model: &BarkModel,
    gas: &mut Option<i32>,
) -> (String, BarkState) {
    match model.chat_completion_create(preferred_model, prompt.into(), &vec![]) {
        Ok(BarkResponse::Chat { mut choices, usage }) => {
            if let Some(gas) = gas {
                *gas = *gas - usage.unwrap_or(1000) as i32;
            }
            if choices.is_empty() {
                eprintln!("Prompt Error (empty)");
                return ("".to_string(), BarkState::Failed);
            } else if choices[0].value.is_empty() {
                eprintln!("Prompt Error (empty message)");
                return ("".to_string(), BarkState::Failed);
            } else if choices.len() > 1 {
                eprintln!("Prompt Warning (multiple choices): {:?}", choices);
            }
            (choices.pop().unwrap().value, BarkState::Complete)
        }
        Ok(BarkResponse::ToolCalls { calls, usage }) => {
            if let Some(gas) = gas {
                *gas = *gas - usage.unwrap_or(1000) as i32;
            }
            eprintln!("Prompt Error (tool calls): {:?}", calls);
            ("".to_string(), BarkState::Failed)
        }
        Err(e) => {
            eprintln!("Prompt Error: {:?}", e);
            ("".to_string(), BarkState::Failed)
        }
    }
}

pub fn powered_chat(
    preferred_model: Option<&String>,
    prompt: Vec<BarkMessage>,
    model: &BarkModel,
    gas: &mut Option<i32>,
    tools: &Vec<BarkTool>,
) -> (String, Vec<BarkMessage>, BarkState) {
    println!("Prompt: {:?}", prompt);
    let response = model.chat_completion_create(preferred_model, prompt.clone().into(), tools);
    println!("Response: {:?}", response);
    match response {
        Ok(BarkResponse::Chat { mut choices, usage }) => {
            if let Some(gas) = gas {
                *gas = *gas - usage.unwrap_or(1000) as i32;
            }
            if choices.is_empty() {
                eprintln!("Prompt Error (empty)");
                return ("".to_string(), prompt, BarkState::Failed);
            } else if choices[0].value.is_empty() {
                eprintln!("Prompt Error (empty message)");
                return ("".to_string(), prompt, BarkState::Failed);
            } else if choices.len() > 1 {
                eprintln!("Prompt Warning (multiple choices): {:?}", choices);
            }
            let response = choices.pop().unwrap();
            let mut messages = prompt.clone();
            messages.push(BarkMessage {
                role: BarkRole::Assistant,
                content: BarkContent::Text(response.value.clone()),
            });
            (response.value, messages, BarkState::Complete)
        }
        Ok(BarkResponse::ToolCalls { calls, usage }) => {
            if let Some(gas) = gas {
                *gas = *gas - usage.unwrap_or(1000) as i32;
            }
            let mut messages = prompt.clone();
            for call in &calls {
                messages.push(BarkMessage {
                    role: BarkRole::Assistant,
                    content: BarkContent::ToolCall(call.clone()),
                });
                match model.call_tool(&call, &prompt) {
                    Ok(BarkToolCallResponse { id, result, .. }) => {
                        if let Some(result) = result {
                            messages.push(BarkMessage {
                                role: BarkRole::Tool,
                                content: BarkContent::ToolResponse {
                                    response: result.clone(),
                                    id: id.clone(),
                                },
                            });
                        } else {
                            eprintln!("Tool call error: {:?}", id);
                            messages.push(BarkMessage {
                                role: BarkRole::Tool,
                                content: BarkContent::ToolResponse {
                                    response: "Tool call error".to_string(),
                                    id: id.clone(),
                                },
                            });
                            return ("".to_string(), messages, BarkState::Failed);
                        }
                    }
                    Err(e) => {
                        eprintln!("Tool call error: {:?}", e);
                        messages.push(BarkMessage {
                            role: BarkRole::Tool,
                            content: BarkContent::Text(format!("Tool call error: {:?}", e)),
                        });
                        return ("".to_string(), messages, BarkState::Failed);
                    }
                }
            }
            return powered_chat(preferred_model, messages, model, gas, tools);
        }
        Err(e) => {
            eprintln!("Prompt Error: {:?}", e);
            ("".to_string(), prompt, BarkState::Failed)
        }
    }
}
