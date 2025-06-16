use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prompt {
    pub ai_model: Option<String>,
    pub prompt: PromptValue,
}

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
        let prompt = controller.get_prompt(&self.prompt);
        if prompt.is_empty() {
            eprintln!("Prompt {:?} is empty", self.prompt);
            return BarkState::Failed;
        }
        let (output, result) = powered_prompt(self.ai_model.as_ref(), prompt.clone(), model, gas);
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
pub struct MatchResponse {
    pub ai_model: Option<String>,
    pub matches: TextMatcher,
    pub prompt: PromptValue,
}

impl BehaviorTree for MatchResponse {
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
            eprintln!("Prompt {:?} is empty", self.prompt);
            return BarkState::Failed;
        }
        let (output, result) = powered_prompt(self.ai_model.as_ref(), prompt.clone(), model, gas);
        check_gas!(gas);
        if result == BarkState::Complete {
            controller
                .text_variables
                .insert(VariableId::LastOutput, output.clone());
            if controller.text_matches(&TextValue::Simple(output), &self.matches) {
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
