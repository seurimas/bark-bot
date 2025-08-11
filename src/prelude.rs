pub use crate::bt::values::{MessageValue, PromptValue, TextMatcher, TextValue, VariableId};
pub use crate::bt::BarkDef;
pub use crate::bt::BarkNode;
pub use crate::bt::{BarkController, BarkFunction, BarkModel, BarkModelConfig, BarkState};
pub use behavior_bark::powered::*;

pub use behavior_bark::check_gas;
use futures::executor::block_on;

pub use crate::clients::*;
pub use std::collections::HashMap;
use std::path::Path;
use tokio::task::JoinHandle;

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

pub fn assistant(s: &impl ToString) -> BarkMessage {
    BarkMessage {
        role: BarkRole::Assistant,
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

pub async fn powered_prompt(
    preferred_model: Option<String>,
    prompt: Vec<BarkMessage>,
    model: BarkModel,
    mut gas: Option<i32>,
) -> (String, BarkState, Option<i32>) {
    match model
        .chat_completion_create(preferred_model, prompt.into(), vec![])
        .await
    {
        Ok(BarkResponse::Chat { mut choices, usage }) => {
            if let Some(gas) = &mut gas {
                *gas = *gas - usage.unwrap_or(1000) as i32;
            }
            if choices.is_empty() {
                // eprintln!("Prompt Error (empty)");
                return ("".to_string(), BarkState::Failed, gas);
            // } else if choices[0].value.is_empty() {
            //     eprintln!("Prompt Error (empty message)");
            //     return ("".to_string(), BarkState::Failed);
            } else if choices.len() > 1 {
                // eprintln!("Prompt Warning (multiple choices): {:?}", choices);
            }
            let response = choices.pop().unwrap().value;
            if response.starts_with("<|start_header_id|>assistant<|end_header_id|>\n") {
                // Handle special case for assistant header
                let response =
                    response.replace("<|start_header_id|>assistant<|end_header_id|>\n", "");
                return (response, BarkState::Complete, gas);
            }
            (response, BarkState::Complete, gas)
        }
        Ok(BarkResponse::ToolCalls { calls, usage }) => {
            if let Some(gas) = &mut gas {
                *gas = *gas - usage.unwrap_or(1000) as i32;
            }
            // eprintln!("Prompt Error (tool calls): {:?}", calls);
            ("".to_string(), BarkState::Failed, gas)
        }
        Err(e) => {
            // eprintln!("Prompt Error: {:?}", e);
            ("".to_string(), BarkState::Failed, gas)
        }
    }
}

pub async fn powered_chat(
    preferred_model: Option<String>,
    mut prompt: Vec<BarkMessage>,
    model: BarkModel,
    mut gas: Option<i32>,
    tools: Vec<BarkTool>,
) -> (String, Vec<BarkMessage>, BarkState, Option<i32>) {
    loop {
        let response = model
            .clone()
            .chat_completion_create(
                preferred_model.clone(),
                prompt.clone().into(),
                tools.clone(),
            )
            .await;
        match response {
            Ok(BarkResponse::Chat { mut choices, usage }) => {
                if let Some(gas) = &mut gas {
                    *gas = *gas - usage.unwrap_or(1000) as i32;
                }
                if choices.is_empty() {
                    // eprintln!("Prompt Error (empty)");
                    return ("".to_string(), prompt, BarkState::Failed, gas);
                // } else if choices[0].value.is_empty() {
                //     eprintln!("Prompt Error (empty message)");
                //     return ("".to_string(), prompt, BarkState::Failed);
                } else if choices.len() > 1 {
                    // eprintln!("Prompt Warning (multiple choices): {:?}", choices);
                }
                let response = choices.pop().unwrap();
                let mut messages = prompt.clone();
                messages.push(BarkMessage {
                    role: BarkRole::Assistant,
                    content: BarkContent::Text(response.value.clone()),
                });
                return (response.value, messages, BarkState::Complete, gas);
            }
            Ok(BarkResponse::ToolCalls { calls, usage }) => {
                if let Some(gas) = &mut gas {
                    *gas = *gas - usage.unwrap_or(1000) as i32;
                }
                let mut messages = prompt.clone();
                for call in &calls {
                    messages.push(BarkMessage {
                        role: BarkRole::Assistant,
                        content: BarkContent::ToolCall(call.clone()),
                    });
                    match model.clone().call_tool(&call, &prompt).await {
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
                                // eprintln!("Tool call error: {:?}", id);
                                messages.push(BarkMessage {
                                    role: BarkRole::Tool,
                                    content: BarkContent::ToolResponse {
                                        response: "Tool call error".to_string(),
                                        id: id.clone(),
                                    },
                                });
                                return ("".to_string(), messages, BarkState::Failed, gas);
                            }
                        }
                        Err(e) => {
                            // eprintln!("Tool call error: {:?}", e);
                            messages.push(BarkMessage {
                                role: BarkRole::Tool,
                                content: BarkContent::Text(format!("Tool call error: {:?}", e)),
                            });
                            return ("".to_string(), messages, BarkState::Failed, gas);
                        }
                    }
                }
                prompt = messages;
            }
            Err(e) => {
                // eprintln!("Prompt Error: {:?}", e);
                return ("".to_string(), prompt, BarkState::Failed, gas);
            }
        }
    }
}

pub fn try_join<T>(handle: &mut JoinHandle<T>) -> std::result::Result<T, ()> {
    if handle.is_finished() {
        match block_on(async { handle.await }) {
            Ok(result) => Ok(result),
            Err(_) => {
                eprintln!("Join handle failed");
                Err(())
            }
        }
    } else {
        Err(())
    }
}
