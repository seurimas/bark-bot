use tokio::task::JoinHandle;

use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct PushSimpleEmbedding<TC: ToolCaller> {
    pub db: TextValue,
    pub text: TextValue,
    #[serde(skip)]
    pub join_handle: Option<JoinHandle<Result<(Vec<f32>, Option<i32>), String>>>,
    #[serde(skip)]
    pub _phantom: std::marker::PhantomData<TC>,
}

impl<TC: ToolCaller> BehaviorTree for PushSimpleEmbedding<TC> {
    type Controller = BarkController;
    type Model = BarkModel<TC>;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
        gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        if let Some(join_handle) = &mut self.join_handle {
            match try_join(join_handle) {
                Ok(result) => {
                    self.join_handle = None;
                    match result {
                        Ok((embedding, new_gas)) => {
                            let db = controller.get_text(&self.db);
                            *gas = new_gas;
                            check_gas!(gas);
                            let text = controller.get_text(&self.text);
                            return match model.push_embedding(db.clone(), text, embedding, None) {
                                Ok(_) => BarkState::Complete,
                                Err(err) => {
                                    // eprintln!("Failed to push simple embedding: {:?}", err);
                                    BarkState::Failed
                                }
                            };
                        }
                        Err(err) => {
                            // eprintln!("Failed to get embedding: {:?}", err);
                            return BarkState::Failed;
                        }
                    }
                }
                Err(join_failed) => {
                    if join_failed {
                        self.join_handle = None; // Clear the join handle on failure
                        return BarkState::Failed;
                    } else {
                        return BarkState::Waiting;
                    }
                }
            }
        }
        let text = controller.get_text(&self.text);
        let model = model.clone();
        self.join_handle = Some(tokio::spawn(model.get_embedding(text, *gas)));
        BarkState::Waiting
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PushValuedEmbedding<TC: ToolCaller> {
    pub db: TextValue,
    pub text: TextValue,
    pub kvs: Vec<(TextValue, TextValue)>,
    #[serde(skip)]
    pub join_handle: Option<JoinHandle<Result<(Vec<f32>, Option<i32>), String>>>,
    #[serde(skip)]
    pub _phantom: std::marker::PhantomData<TC>,
}

impl<TC: ToolCaller> BehaviorTree for PushValuedEmbedding<TC> {
    type Controller = BarkController;
    type Model = BarkModel<TC>;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
        gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        if let Some(join_handle) = &mut self.join_handle {
            match try_join(join_handle) {
                Ok(result) => {
                    self.join_handle = None; // Clear the join handle after completion
                    match result {
                        Ok((embedding, new_gas)) => {
                            let db = controller.get_text(&self.db);
                            let key_values = self
                                .kvs
                                .iter()
                                .map(|(k, v)| (controller.get_text(k), controller.get_text(v)))
                                .collect();
                            *gas = new_gas;
                            check_gas!(gas);
                            let text = controller.get_text(&self.text);
                            return match model.push_embedding(db, text, embedding, Some(key_values))
                            {
                                Ok(_) => BarkState::Complete,
                                Err(_) => BarkState::Failed,
                            };
                        }

                        Err(err) => {
                            // eprintln!("Failed to get embedding: {:?}", err);
                            return BarkState::Failed;
                        }
                    }
                }
                Err(join_failed) => {
                    if join_failed {
                        self.join_handle = None; // Clear the join handle on failure
                        return BarkState::Failed;
                    } else {
                        return BarkState::Waiting;
                    }
                }
            }
        }

        let text = controller.get_text(&self.text);
        let model = model.clone();
        self.join_handle = Some(tokio::spawn(model.get_embedding(text, *gas)));
        BarkState::Waiting
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
