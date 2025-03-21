use behavior_bark::powered::BehaviorTree;
use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub prompt: PromptValue,
    pub tools: Vec<String>,
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
        let (output, result) = powered_prompt(None, prompt.clone(), model, gas, vec![]);
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
