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
            Ok(embedding) => match model.push_embedding(self.0.clone(), text, embedding, None) {
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
pub struct PushValuedEmbedding(pub String, pub TextValue, pub Vec<(TextValue, TextValue)>);

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
        let embedding = model.get_embedding(&text, gas);
        let key_values = self
            .2
            .iter()
            .map(|(k, v)| (controller.get_text(k), controller.get_text(v)))
            .collect();
        check_gas!(gas);
        match embedding {
            Ok(embedding) => {
                match model.push_embedding(self.0.clone(), text, embedding, Some(key_values)) {
                    Ok(_) => BarkState::Complete,
                    Err(_) => BarkState::Failed,
                }
            }
            Err(_) => BarkState::Failed,
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
