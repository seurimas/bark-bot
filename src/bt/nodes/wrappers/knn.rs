use crate::prelude::*;
use tokio::task::JoinHandle;

pub struct Knn {
    path: String,
    compared: TextValue,
    k: usize,
    current: usize,
    results: Vec<String>,
    node: Box<dyn BehaviorTree<Model = BarkModel, Controller = BarkController> + Send + Sync>,
    join_handle: Option<JoinHandle<Result<(Vec<f32>, Option<i32>), String>>>,
}

impl Knn {
    pub fn new(
        path: String,
        compared: TextValue,
        k: usize,
        mut nodes: Vec<
            Box<dyn BehaviorTree<Model = BarkModel, Controller = BarkController> + Send + Sync>,
        >,
    ) -> Self {
        Self {
            path,
            compared,
            k,
            current: 0,
            results: vec![],
            node: nodes.pop().unwrap(),
            join_handle: None,
        }
    }
}

impl BehaviorTree for Knn {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
        gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        if self.results.is_empty() && self.join_handle.is_none() {
            let compared_text = controller.get_text(&self.compared);
            let model = model.clone();
            self.join_handle = Some(tokio::spawn(model.get_embedding(compared_text, *gas)));
            return BarkState::Waiting;
        } else if let Some(join_handle) = &mut self.join_handle {
            if let Ok(result) = try_join(join_handle) {
                self.join_handle = None; // Clear the join handle after completion
                if let Ok(result) = result {
                    let compared_embedding = result.0;
                    *gas = result.1;
                    check_gas!(gas);
                    match model.pull_best_matches(&self.path, compared_embedding, self.k) {
                        Ok(results) => {
                            if results.is_empty() {
                                return BarkState::Failed;
                            }
                            self.results = results;
                            self.current = 0;
                        }
                        Err(_) => {
                            return BarkState::Failed;
                        }
                    }
                } else {
                    return BarkState::Failed;
                }
            } else {
                return BarkState::Waiting;
            }
        }
        while self.current < self.results.len() {
            let text_value = self.results[self.current].clone();
            controller
                .text_variables
                .insert(VariableId::LoopValue, text_value);
            match self.node.resume_with(model, controller, gas, _audit) {
                BarkState::Complete => {
                    self.node.reset(model);
                    self.current = self.current + 1;
                }
                BarkState::Waiting => {
                    return BarkState::Waiting;
                }
                BarkState::Failed => {
                    return BarkState::Failed;
                }
                BarkState::WaitingForGas => {
                    return BarkState::WaitingForGas;
                }
            }
        }
        BarkState::Complete
    }

    fn reset(self: &mut Self, model: &Self::Model) {
        self.current = 0;
        self.results = vec![];
        self.node.reset(model);
    }
}
