use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PullBestScored(pub String, pub TextValue);

impl BehaviorTree for PullBestScored {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
        gas: &mut Option<i32>,
        mut audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        println!("PullBestScored: {}", self.0);
        audit.enter(&"PullBestScored");
        let text = controller.get_text(&self.1);
        let embedding = model.get_embedding(&text, gas);
        check_gas!(gas);
        match embedding {
            Ok(embedding) => {
                if let Ok(best_match) = model.pull_best_match(&self.0, embedding) {
                    controller
                        .text_variables
                        .insert(VariableId::LastOutput, best_match);
                    audit.mark(&format!("Pulled best match for: {}", self.0));
                    audit.exit(&"PullBestScored", BarkState::Complete);
                    BarkState::Complete
                } else {
                    eprintln!("Failed to pull best match");
                    audit.mark(&format!("Failed to pull best match for: {}", self.0));
                    audit.exit(&"PullBestScored", BarkState::Failed);
                    BarkState::Failed
                }
            }
            Err(err) => {
                audit.mark(&format!("Failed to get embedding: {}", err));
                audit.exit(&"PullBestScored", BarkState::Failed);
                BarkState::Failed
            }
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
