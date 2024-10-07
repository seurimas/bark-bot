use crate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushSimpleEmbedding(pub String, pub TextValue);

impl BehaviorTree for PushSimpleEmbedding {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
        gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        let text = controller.get_text(&self.1);
        let embedding = model.get_embedding(&text, gas);
        check_gas!(gas);
        match embedding {
            Ok(embedding) => match model.push_embedding(self.0.clone(), text, embedding) {
                Ok(_) => BarkState::Complete,
                Err(err) => {
                    eprintln!("Failed to push simple embedding: {:?}", err);
                    BarkState::Failed
                }
            },
            Err(_) => BarkState::Failed,
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushValuedEmbedding(pub String, pub TextValue, pub TextValue);

impl BehaviorTree for PushValuedEmbedding {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
        gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        let text = controller.get_text(&self.1);
        let embedding_text = controller.get_text(&self.2);
        let embedding = model.get_embedding(&embedding_text, gas);
        check_gas!(gas);
        match embedding {
            Ok(embedding) => match model.push_embedding(self.0.clone(), text, embedding) {
                Ok(_) => BarkState::Complete,
                Err(_) => BarkState::Failed,
            },
            Err(_) => BarkState::Failed,
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
