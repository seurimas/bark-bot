use std::sync::atomic::AtomicUsize;

use behavior_bark::powered::BehaviorTree;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

use crate::prelude::*;

static PROMPT_IDS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Serialize, Deserialize)]
pub struct Agent<TC: ToolCaller> {
    pub ai_model: Option<TextValue>,
    pub prompt: PromptValue,
    pub tool_filters: TextValue,
    #[serde(skip)]
    pub join_handle: Option<
        JoinHandle<
            Result<
                (String, Vec<BarkMessage>, BarkState, Option<i32>),
                (String, Vec<BarkMessage>, Option<i32>),
            >,
        >,
    >,
    #[serde(skip)]
    pub prompt_id: Option<usize>,
    #[serde(skip)]
    pub _phantom: std::marker::PhantomData<TC>,
}

impl<TC: ToolCaller> BehaviorTree for Agent<TC> {
    type Controller = BarkController;
    type Model = BarkModel<TC>;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
        gas: &mut Option<i32>,
        mut audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        check_gas!(gas);
        if let (Some(id), Some(join_handle)) = (&self.prompt_id, &mut self.join_handle) {
            match try_join(join_handle) {
                Ok(result) => {
                    self.join_handle = None;
                    match result {
                        Ok((output, chat, result, new_gas)) => {
                            *gas = new_gas;
                            audit.data(&"Prompt", &format!("output-{}", id), &output);
                            if result == BarkState::Complete {
                                controller
                                    .text_variables
                                    .insert(VariableId::LastOutput, output);
                                controller.prompts.insert(VariableId::LastOutput, chat);
                            }
                            return result;
                        }
                        Err((err, chat, new_gas)) => {
                            *gas = new_gas;
                            audit.data(&"Prompt", &format!("error-chat-{}", id), &chat);
                            controller.prompts.insert(VariableId::LastOutput, chat);
                            audit.data(&"Prompt", &format!("error-{}", id), &err);
                            return BarkState::Failed;
                        }
                    }
                }
                Err(join_failed) => {
                    if join_failed {
                        self.join_handle = None; // Clear the join handle on failure
                        audit.data(&"Prompt", &format!("error-{}", id), &"Join failed");
                        return BarkState::Failed;
                    } else {
                        return BarkState::Waiting;
                    }
                }
            }
        }
        let prompt = controller.get_prompt(&self.prompt);
        let prompt_id = PROMPT_IDS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.prompt_id = Some(prompt_id);
        if prompt.is_empty() {
            return BarkState::Failed;
        }
        audit.data(&"Prompt", &format!("prompt-{}", prompt_id), &prompt);
        let tool_filters_text = controller.get_text(&self.tool_filters);
        let tool_filters: Vec<String> = tool_filters_text
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let tools = model.get_tools(&tool_filters);
        let ai_model = self.ai_model.as_ref().map(|v| controller.get_text(v));
        self.join_handle = Some(tokio::spawn(powered_chat(
            ai_model,
            prompt.clone(),
            model.clone(),
            *gas,
            tools.clone(),
        )));
        BarkState::Waiting
    }
    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
