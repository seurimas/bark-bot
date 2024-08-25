use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Search(pub TextValue);

impl UnpoweredFunction for Search {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        let text = controller.get_text(&self.0);
        let results = model.search(&text);
        UnpoweredFunctionState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
