use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReadStdio(pub bool, pub VariableId);

impl UnpoweredFunction for ReadStdio {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        let value = model.read_stdin(self.0);
        controller.text_variables.insert(self.1.clone(), value);
        UnpoweredFunctionState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrintLine(pub TextValue);

impl UnpoweredFunction for PrintLine {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        println!("{}", controller.get_text(&self.0));
        UnpoweredFunctionState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AskForInput(pub TextValue);

impl UnpoweredFunction for AskForInput {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        println!("{}", controller.get_text(&self.0));
        let value = model.read_stdin(true);
        controller
            .text_variables
            .insert(VariableId::LastOutput, value);
        UnpoweredFunctionState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
