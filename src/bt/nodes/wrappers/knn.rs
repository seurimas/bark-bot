use crate::prelude::*;

pub struct Knn {
    path: String,
    compared: TextValue,
    k: usize,
    current: usize,
    results: Vec<String>,
    node: Box<dyn UnpoweredFunction<Model = BarkModel, Controller = BarkController> + Send + Sync>,
}

impl Knn {
    pub fn new(
        path: String,
        compared: TextValue,
        k: usize,
        mut nodes: Vec<
            Box<
                dyn UnpoweredFunction<Model = BarkModel, Controller = BarkController> + Send + Sync,
            >,
        >,
    ) -> Self {
        Self {
            path,
            compared,
            k,
            current: 0,
            results: vec![],
            node: nodes.pop().unwrap(),
        }
    }
}

impl UnpoweredFunction for Knn {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        if self.results.is_empty() {
            let compared_text = controller.get_text(&self.compared);
            let compared_embedding = model.get_embedding(&compared_text);
            if let Ok(compared_embedding) = compared_embedding {
                match model.pull_best_matches(&self.path, compared_embedding, self.k) {
                    Ok(results) => {
                        self.results = results;
                        self.current = 0;
                    }
                    Err(_) => {
                        return UnpoweredFunctionState::Failed;
                    }
                }
            } else {
                return UnpoweredFunctionState::Failed;
            }
        }
        while self.current < self.results.len() {
            let text_value = self.results[self.current].clone();
            controller
                .text_variables
                .insert(VariableId::LoopValue, text_value);
            match self.node.resume_with(model, controller) {
                UnpoweredFunctionState::Complete => {
                    self.current = self.current + 1;
                }
                UnpoweredFunctionState::Waiting => {
                    return UnpoweredFunctionState::Waiting;
                }
                UnpoweredFunctionState::Failed => {
                    return UnpoweredFunctionState::Failed;
                }
            }
        }
        UnpoweredFunctionState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        self.current = 0;
        self.results = vec![];
    }
}
