use crate::prelude::*;

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
        let embedding = model.get_embedding(text);
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
