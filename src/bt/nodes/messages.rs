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
pub struct AddUserMessage(pub VariableId, pub String);

impl UnpoweredFunction for AddUserMessage {
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
            .map(|prompt| prompt.push(user(&self.1)));
        UnpoweredFunctionState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddSystemMessage(pub VariableId, pub String);

impl UnpoweredFunction for AddSystemMessage {
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
            .map(|prompt| prompt.push(system(&self.1)));
        UnpoweredFunctionState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddUserFromVariable(pub VariableId, pub VariableId);

impl UnpoweredFunction for AddUserFromVariable {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        let from = controller.text_variables.get(&self.1);
        if let Some(from) = from {
            controller.prompts.get_mut(&self.0).map(|prompt| {
                prompt.push(user(from));
            });
        }
        UnpoweredFunctionState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AddSystemFromVariable(pub VariableId, pub VariableId);

impl UnpoweredFunction for AddSystemFromVariable {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        let from = controller.text_variables.get(&self.1);
        if let Some(from) = from {
            controller.prompts.get_mut(&self.0).map(|prompt| {
                prompt.push(system(from));
            });
        }
        UnpoweredFunctionState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
