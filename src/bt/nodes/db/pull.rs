use tokio::task::JoinHandle;

use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct PullBestScored {
    pub db: TextValue,
    pub text: TextValue,
    #[serde(skip)]
    pub join_handle: Option<JoinHandle<Result<(Vec<f32>, Option<i32>), String>>>,
}

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
        if let Some(join_handle) = &mut self.join_handle {
            if let Ok(result) = try_join(join_handle) {
                self.join_handle = None;
                match result {
                    Ok((embedding, new_gas)) => {
                        let db = controller.get_text(&self.db);
                        *gas = new_gas;
                        check_gas!(gas);
                        if let Ok(best_match) = model.pull_best_match(&db, embedding) {
                            controller
                                .text_variables
                                .insert(VariableId::LastOutput, best_match);
                            audit.mark(&format!("Pulled best match for: {}", db));
                            audit.exit(&"PullBestScored", BarkState::Complete);
                            return BarkState::Complete;
                        } else {
                            // eprintln!("Failed to pull best match");
                            audit.mark(&format!("Failed to pull best match for: {}", db));
                            audit.exit(&"PullBestScored", BarkState::Failed);
                            return BarkState::Failed;
                        }
                    }
                    Err(err) => {
                        audit.mark(&format!("Failed to get embedding: {}", err));
                        audit.exit(&"PullBestScored", BarkState::Failed);
                        return BarkState::Failed;
                    }
                }
            } else {
                return BarkState::Waiting;
            }
        }
        audit.enter(&"PullBestScored");
        let text = controller.get_text(&self.text);
        let model = model.clone();
        self.join_handle = Some(tokio::spawn(model.get_embedding(text, *gas)));
        BarkState::Waiting
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
