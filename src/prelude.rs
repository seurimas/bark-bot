use crate::bt::strip_thoughts;
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

pub fn read_tree<TC: ToolCaller>(root: impl AsRef<Path>, tree_path: &str) -> BarkDef<TC> {
    let root = root.as_ref();
    let path = std::path::Path::join(root, tree_path);
    let tree = std::fs::read_to_string(&path)
        .expect(format!("Failed to read tree file: {:?}", path).as_str());
    let tree: crate::bt::BarkDef<TC> = if tree_path.ends_with("json") {
        serde_json::from_str(&tree).expect("Failed to parse JSON tree file")
    } else if tree_path.ends_with("ron") {
        ron::from_str(&tree).expect("Failed to parse RON tree file")
    } else {
        panic!("Unsupported tree file format: {:?}", tree_path)
    };
    tree
}

pub async fn powered_prompt<TC: ToolCaller>(
    preferred_model: Option<String>,
    prompt: Vec<BarkMessage>,
    model: BarkModel<TC>,
    mut gas: Option<i32>,
) -> Result<(String, BarkState, Option<i32>), (String, Option<i32>)> {
    match model
        .chat_completion_create(preferred_model, prompt.into(), vec![])
        .await
    {
        Ok(BarkResponse::Chat { mut choices, usage }) => {
            if let Some(gas) = &mut gas {
                *gas = *gas - usage.unwrap_or(1000) as i32;
            }
            if choices.is_empty() {
                return Err(("Empty response from model".to_string(), gas));
            } else if choices[0].value.is_empty() {
                return Err(("Empty message from model".to_string(), gas));
            } else if choices.len() > 1 {
                return Err(("Multiple choices returned from model".to_string(), gas));
            }
            let response = choices.pop().unwrap().value;
            if response.starts_with("<|start_header_id|>assistant<|end_header_id|>\n") {
                // Handle special case for assistant header
                let response =
                    response.replace("<|start_header_id|>assistant<|end_header_id|>\n", "");
                return Ok((response, BarkState::Complete, gas));
            }
            Ok((response, BarkState::Complete, gas))
        }
        Ok(BarkResponse::ToolCalls { calls, usage }) => {
            if let Some(gas) = &mut gas {
                *gas = *gas - usage.unwrap_or(1000) as i32;
            }
            Err((format!("Unexpected tool calls in prompt: {:?}", calls), gas))
        }
        Err(e) => Err((format!("Error from model: {:?}", e), gas)),
    }
}

pub async fn powered_chat<TC: ToolCaller>(
    preferred_model: Option<String>,
    mut prompt: Vec<BarkMessage>,
    model: BarkModel<TC>,
    mut gas: Option<i32>,
    tools: Vec<BarkTool>,
) -> Result<
    (String, Vec<BarkMessage>, BarkState, Option<i32>),
    (String, Vec<BarkMessage>, Option<i32>),
> {
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
                    return Err(("Empty response from model".to_string(), prompt, gas));
                } else if choices[0].value.is_empty() {
                    return Err(("Empty message from model".to_string(), prompt, gas));
                } else if choices.len() > 1 {
                    return Err((
                        "Multiple choices returned from model".to_string(),
                        prompt,
                        gas,
                    ));
                }
                let response = choices.pop().unwrap();
                let mut messages = prompt.clone();
                let value = if model.strip_thoughts_in_chat {
                    strip_thoughts(&response.value)
                } else {
                    response.value.clone()
                };
                messages.push(BarkMessage {
                    role: BarkRole::Assistant,
                    content: BarkContent::Text(value),
                });
                return Ok((response.value, messages, BarkState::Complete, gas));
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
                                return Err((
                                    "Tool call returned no result".to_string(),
                                    messages,
                                    gas,
                                ));
                            }
                        }
                        Err(e) => {
                            return Err((format!("Tool call failed: {}", e), messages, gas));
                        }
                    }
                }
                prompt = messages;
            }
            Err(e) => {
                return Err((
                    format!(
                        "Error from model: {:?}, last_message: {:?}",
                        e,
                        prompt[prompt.len() - 1],
                    ),
                    prompt,
                    gas,
                ));
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
