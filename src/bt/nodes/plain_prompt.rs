use std::sync::atomic::AtomicUsize;

use crate::prelude::*;
use tokio::task::JoinHandle;

static PROMPT_IDS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Serialize, Deserialize)]
pub struct Prompt {
    pub ai_model: Option<TextValue>,
    pub prompt: PromptValue,
    #[serde(skip)]
    pub join_handle: Option<JoinHandle<(String, BehaviorTreeState, Option<i32>)>>,
    #[serde(skip)]
    pub prompt_id: Option<usize>,
}

impl BehaviorTree for Prompt {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
        gas: &mut Option<i32>,
        mut audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        check_gas!(gas);
        if let (Some(id), Some(join_handle)) = (&self.prompt_id, &mut self.join_handle) {
            if let Ok(result) = try_join(join_handle) {
                self.join_handle = None;
                let (output, result, new_gas) = result;
                *gas = new_gas;
                audit.data(&"Prompt", &format!("output-{}", id), &output);
                if result == BarkState::Complete {
                    let mut prompt = controller.get_prompt(&self.prompt);
                    prompt.push(BarkMessage {
                        role: BarkRole::Assistant,
                        content: BarkContent::Text(output.clone()),
                    });
                    controller
                        .prompts
                        .insert(VariableId::LastOutput, prompt.clone());

                    controller
                        .text_variables
                        .insert(VariableId::LastOutput, output);
                }
                return result;
            } else {
                return BarkState::Waiting;
            }
        }
        let prompt = controller.get_prompt(&self.prompt);
        let prompt_id = PROMPT_IDS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.prompt_id = Some(prompt_id);
        audit.data(&"Prompt", &format!("prompt-{}", prompt_id), &prompt);
        if prompt.is_empty() {
            // eprintln!("Prompt {:?} is empty", self.prompt);
            return BarkState::Failed;
        }
        let ai_model = self.ai_model.as_ref().map(|v| controller.get_text(v));
        self.join_handle = Some(tokio::spawn(powered_prompt(
            ai_model,
            prompt.clone(),
            model.clone(),
            *gas,
        )));
        BarkState::Waiting
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
        self.join_handle = None;
        self.prompt_id = None;
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MatchResponse {
    pub ai_model: Option<TextValue>,
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
        mut audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        if let Some(join_handle) = &mut self.join_handle {
            if let Ok(result) = try_join(join_handle) {
                self.join_handle = None; // Clear the join handle after completion
                let (output, result, new_gas) = result;
                *gas = new_gas;
                check_gas!(gas);
                if result == BarkState::Complete {
                    controller
                        .text_variables
                        .insert(VariableId::LastOutput, output.clone());
                    audit.data(&"MatchResponse", &"output", &output);
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
        audit.data(&"MatchResponse", &"prompt", &prompt);
        if prompt.is_empty() {
            // eprintln!("Prompt {:?} is empty", self.prompt);
            return BarkState::Failed;
        }
        let ai_model = self.ai_model.as_ref().map(|v| controller.get_text(v));
        self.join_handle = Some(tokio::spawn(powered_prompt(
            ai_model,
            prompt.clone(),
            model.clone(),
            *gas,
        )));
        BarkState::Waiting
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        self.join_handle = None;
    }
}
