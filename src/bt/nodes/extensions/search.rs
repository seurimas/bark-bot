use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Search(pub TextValue);

impl BehaviorTree for Search {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
        _gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        let text = controller.get_text(&self.0);
        let results = model.search(&text);
        BarkState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
