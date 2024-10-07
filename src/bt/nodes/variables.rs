use crate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetEmbedding(pub TextValue, pub VariableId);

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
        let text = controller.get_text(&self.0);
        let embedding = model.get_embedding(&text, gas);
        check_gas!(gas);
        match embedding {
            Ok(embedding) => {
                controller
                    .embedding_variables
                    .insert(self.1.clone(), embedding);
                BarkState::Complete
            }
            Err(_) => BarkState::Failed,
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StartPrompt(pub VariableId, pub Vec<MessageValue>);

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExtendPrompt(pub VariableId, pub Vec<MessageValue>);

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
