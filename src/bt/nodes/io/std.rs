use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReadStdio(pub bool, pub VariableId);

impl BehaviorTree for ReadStdio {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
        _gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        let value = model.read_stdin(self.0);
        controller.text_variables.insert(self.1.clone(), value);
        BarkState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrintLine(pub TextValue);

impl BehaviorTree for PrintLine {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
        _gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        println!("{}", controller.get_text(&self.0));
        BarkState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AskForInput(pub TextValue);

impl BehaviorTree for AskForInput {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
        _gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        println!("{}", controller.get_text(&self.0));
        let value = model.read_stdin(true);
        controller
            .text_variables
            .insert(VariableId::LastOutput, value);
        BarkState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
