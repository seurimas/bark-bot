use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadStdio<TC: ToolCaller>(
    pub bool,
    pub VariableId,
    #[serde(skip)] pub std::marker::PhantomData<TC>,
);

impl<TC: ToolCaller> BehaviorTree for ReadStdio<TC> {
    type Controller = BarkController;
    type Model = BarkModel<TC>;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct PrintLine<TC: ToolCaller>(
    pub TextValue,
    #[serde(skip)] pub std::marker::PhantomData<TC>,
);

impl<TC: ToolCaller> BehaviorTree for PrintLine<TC> {
    type Controller = BarkController;
    type Model = BarkModel<TC>;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct AskForInput<TC: ToolCaller>(
    pub TextValue,
    #[serde(skip)] pub std::marker::PhantomData<TC>,
);

impl<TC: ToolCaller> BehaviorTree for AskForInput<TC> {
    type Controller = BarkController;
    type Model = BarkModel<TC>;

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
