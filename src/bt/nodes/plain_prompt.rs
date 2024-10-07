use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prompt(pub PromptValue);

impl BehaviorTree for Prompt {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
        gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        let prompt = controller.get_prompt(&self.0);
        if prompt.is_empty() {
            return BarkState::Failed;
        }
        let (output, result) = powered_prompt(prompt.clone(), model, gas);
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequireInResponse(pub Vec<String>, pub PromptValue);

impl BehaviorTree for RequireInResponse {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
        gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        let prompt = controller.get_prompt(&self.1);
        if prompt.is_empty() {
            return BarkState::Failed;
        }
        let (output, result) = powered_prompt(prompt.clone(), model, gas);
        check_gas!(gas);
        if result == BarkState::Complete {
            controller
                .text_variables
                .insert(VariableId::LastOutput, output.clone());
            if self.0.iter().any(|s| output.to_lowercase().contains(s)) {
                BarkState::Complete
            } else {
                BarkState::Failed
            }
        } else {
            result
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RejectInResponse(pub Vec<String>, pub PromptValue);

impl BehaviorTree for RejectInResponse {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
        gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        let prompt = controller.get_prompt(&self.1);
        if prompt.is_empty() {
            return BarkState::Failed;
        }
        let (output, result) = powered_prompt(prompt.clone(), model, gas);
        check_gas!(gas);
        if result == BarkState::Complete {
            controller
                .text_variables
                .insert(VariableId::LastOutput, output.clone());
            if self.0.iter().any(|s| output.to_lowercase().contains(s)) {
                BarkState::Failed
            } else {
                BarkState::Complete
            }
        } else {
            result
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
