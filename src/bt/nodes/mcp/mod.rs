use behavior_bark::powered::BehaviorTree;
use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub prompt: PromptValue,
    pub tool_filters: Vec<String>,
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
        let prompt = controller.get_prompt(&self.prompt);
        if prompt.is_empty() {
            return BarkState::Failed;
        }
        let tools = model.get_tools(&self.tool_filters);
        let (output, last_messages, result) =
            powered_chat(None, prompt.clone(), model, gas, &tools);
        controller
            .prompts
            .insert(VariableId::LastOutput, last_messages);
        check_gas!(gas);
        if result == BarkState::Complete {
            controller
                .text_variables
                .insert(VariableId::LastOutput, output);
        }
        result
    }
    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
