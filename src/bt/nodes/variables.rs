use tokio::task::JoinHandle;

use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct SetText(pub VariableId, pub TextValue);

impl BehaviorTree for SetText {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
        _gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        let text = controller.get_text(&self.1);
        controller.text_variables.insert(self.0.clone(), text);
        BarkState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetTemplate(pub VariableId, pub Vec<MessageValue>);

impl BehaviorTree for SetTemplate {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
        _gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        controller.templates.insert(self.0.clone(), self.1.clone());
        BarkState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetEmbedding {
    pub text: TextValue,
    pub variable: VariableId,
    #[serde(skip)]
    pub join_handle: Option<JoinHandle<Result<(Vec<f32>, Option<i32>), String>>>,
}

impl BehaviorTree for GetEmbedding {
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
                self.join_handle = None; // Clear the join handle after completion
                match result {
                    Ok((embedding, new_gas)) => {
                        *gas = new_gas;
                        controller
                            .embedding_variables
                            .insert(self.variable.clone(), embedding);
                        return BarkState::Complete;
                    }
                    Err(_) => {
                        return BarkState::Failed;
                    }
                }
            }
            return BarkState::Waiting;
        }
        let text = controller.get_text(&self.text);
        let model = model.clone();
        self.join_handle = Some(tokio::spawn(model.get_embedding(text, *gas)));
        BarkState::Waiting
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartPrompt(pub VariableId, pub PromptValue);

impl BehaviorTree for StartPrompt {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
        _gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        controller.start_prompt(self.0.clone(), self.1.clone());
        BarkState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtendPrompt(pub VariableId, pub PromptValue);

impl BehaviorTree for ExtendPrompt {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
        _gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        controller.extend_prompt(self.0.clone(), self.1.clone());
        BarkState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Unescape(pub VariableId);

impl BehaviorTree for Unescape {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
        _gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        let text = controller.text_variables.get(&self.0).unwrap();
        let unescaped = serde_json::from_str(text);
        if unescaped.is_err() {
            return BarkState::Failed;
        }
        let unescaped: String = unescaped.unwrap();
        controller.text_variables.insert(self.0.clone(), unescaped);
        BarkState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
