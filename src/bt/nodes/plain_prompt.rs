use crate::prelude::*;
use tokio::task::JoinHandle;

#[derive(Debug, Serialize, Deserialize)]
pub struct Prompt {
    pub ai_model: Option<String>,
    pub prompt: PromptValue,
    #[serde(skip)]
    pub join_handle: Option<JoinHandle<(String, BehaviorTreeState, Option<i32>)>>,
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
        if let Some(join_handle) = &mut self.join_handle {
            if let Ok(result) = try_join(join_handle) {
                let (output, result, new_gas) = result;
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
            eprintln!("Prompt {:?} is empty", self.prompt);
            return BarkState::Failed;
        }
        self.join_handle = Some(tokio::spawn(powered_prompt(
            self.ai_model.clone(),
            prompt.clone(),
            model.clone(),
            *gas,
        )));
        BarkState::Waiting
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MatchResponse {
    pub ai_model: Option<String>,
    pub matches: TextMatcher,
    pub prompt: PromptValue,
    #[serde(skip)]
    pub join_handle: Option<JoinHandle<(String, BarkState, Option<i32>)>>,
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
        if let Some(join_handle) = &mut self.join_handle {
            if let Ok(result) = try_join(join_handle) {
                let (output, result, new_gas) = result;
                *gas = new_gas;
                check_gas!(gas);
                if result == BarkState::Complete {
                    controller
                        .text_variables
                        .insert(VariableId::LastOutput, output.clone());
                    if controller.text_matches(&TextValue::Simple(output), &self.matches) {
                        return BarkState::Complete;
                    } else {
                        return BarkState::Failed;
                    }
                } else {
                    return result;
                }
            } else {
                return BarkState::Waiting;
            }
        }
        let prompt = controller.get_prompt(&self.prompt);
        if prompt.is_empty() {
            eprintln!("Prompt {:?} is empty", self.prompt);
            return BarkState::Failed;
        }
        self.join_handle = Some(tokio::spawn(powered_prompt(
            self.ai_model.clone(),
            prompt.clone(),
            model.clone(),
            *gas,
        )));
        BarkState::Waiting
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
