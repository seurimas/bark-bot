use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PullBestMatch(pub String, pub TextValue);

impl UnpoweredFunction for PullBestMatch {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        let text = controller.get_text(&self.1);
        let embedding = model.get_embedding(&text);
        match embedding {
            Ok(embedding) => {
                if let Ok(best_match) = model.pull_best_match(self.0.clone(), embedding) {
                    controller
                        .text_variables
                        .insert(VariableId::LastOutput, best_match);
                    UnpoweredFunctionState::Complete
                } else {
                    eprintln!("Failed to pull best match");
                    UnpoweredFunctionState::Failed
                }
            }
            Err(_) => UnpoweredFunctionState::Failed,
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
