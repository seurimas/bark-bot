use crate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetText(pub VariableId, pub TextValue);

impl UnpoweredFunction for SetText {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        let text = controller.get_text(&self.1);
        controller.text_variables.insert(self.0.clone(), text);
        UnpoweredFunctionState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetEmbedding(pub TextValue, pub VariableId);

impl UnpoweredFunction for GetEmbedding {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        let text = controller.get_text(&self.0);
        let embedding = model.get_embedding(&text);
        match embedding {
            Ok(embedding) => {
                controller
                    .embedding_variables
                    .insert(self.1.clone(), embedding);
                UnpoweredFunctionState::Complete
            }
            Err(_) => UnpoweredFunctionState::Failed,
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StartPrompt(pub VariableId, pub Vec<MessageValue>);

impl UnpoweredFunction for StartPrompt {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        controller.start_prompt(self.0.clone(), self.1.clone());
        UnpoweredFunctionState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExtendPrompt(pub VariableId, pub Vec<MessageValue>);

impl UnpoweredFunction for ExtendPrompt {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        controller.extend_prompt(self.0.clone(), self.1.clone());
        UnpoweredFunctionState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
