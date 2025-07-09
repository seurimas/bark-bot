use behavior_bark::powered::BehaviorTree;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct Agent {
    pub ai_model: Option<String>,
    pub prompt: PromptValue,
    pub tool_filters: Vec<String>,
    #[serde(skip)]
    pub join_handle: Option<JoinHandle<(String, Vec<BarkMessage>, BarkState, Option<i32>)>>,
}

impl BehaviorTree for Agent {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
        gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        if let Some(join_handle) = &mut self.join_handle {
            if let Ok((output, _, result, new_gas)) = try_join(join_handle) {
                self.join_handle = None;
                *gas = new_gas;
                check_gas!(gas);
                if result == BarkState::Complete {
                    controller
                        .text_variables
                        .insert(VariableId::LastOutput, output.clone());
                }
                return result;
            } else {
                return BarkState::Waiting;
            }
        }
        let prompt = controller.get_prompt(&self.prompt);
        if prompt.is_empty() {
            return BarkState::Failed;
        }
        let tools = model.get_tools(&self.tool_filters);
        self.join_handle = Some(tokio::spawn(powered_chat(
            self.ai_model.clone(),
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
