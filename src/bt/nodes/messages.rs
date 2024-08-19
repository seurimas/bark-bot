use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResetMessages(pub VariableId);

impl UnpoweredFunction for ResetMessages {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        controller
            .prompts
            .get_mut(&self.0)
            .map(|prompt| prompt.clear());
        UnpoweredFunctionState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddUserMessage(pub VariableId, pub TextValue);

impl UnpoweredFunction for AddUserMessage {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        controller.add_user_to_prompt(self.0.clone(), self.1.clone());
        UnpoweredFunctionState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddSystemMessage(pub VariableId, pub TextValue);

impl UnpoweredFunction for AddSystemMessage {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        controller.add_system_to_prompt(self.0.clone(), self.1.clone());
        UnpoweredFunctionState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
