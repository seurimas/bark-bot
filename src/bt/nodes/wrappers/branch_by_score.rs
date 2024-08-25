use crate::prelude::*;

pub struct BranchByScore {
    compared: TextValue,
    compared_embedding: Vec<f32>,
    text_values: Vec<TextValue>,
    best_index: Option<usize>,
    nodes: Vec<
        Box<dyn UnpoweredFunction<Model = BarkModel, Controller = BarkController> + Send + Sync>,
    >,
}

impl BranchByScore {
    pub fn new(
        compared: TextValue,
        text_values: Vec<TextValue>,
        nodes: Vec<
            Box<
                dyn UnpoweredFunction<Model = BarkModel, Controller = BarkController> + Send + Sync,
            >,
        >,
    ) -> Self {
        if nodes.len() != text_values.len() {
            panic!("BranchByScore nodes and text_values must have the same length");
        }
        Self {
            compared,
            compared_embedding: vec![],
            text_values,
            best_index: None,
            nodes,
        }
    }
}

impl UnpoweredFunction for BranchByScore {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        if self.compared_embedding.is_empty() {
            let compared_text = controller.get_text(&self.compared);
            let embedding = model.get_embedding(&compared_text);
            if let Ok(embedding) = embedding {
                self.compared_embedding = embedding;
            } else {
                return UnpoweredFunctionState::Failed;
            }
        }
        if self.best_index.is_none() {
            let mut best_score = f32::INFINITY;
            let mut best_index = 0;
            for (index, text_value) in self.text_values.iter().enumerate() {
                let text_value = controller.get_text(text_value);
                let score = model
                    .get_embedding(&text_value)
                    .map(|embedding| score(&self.compared_embedding, &embedding));
                if let Ok(score) = score {
                    if score < best_score {
                        best_score = score;
                        best_index = index;
                    }
                }
            }
            self.best_index = Some(best_index);
        }
        if let Some(best_index) = self.best_index {
            let matched = controller.get_text(&self.text_values[best_index]);
            controller
                .text_variables
                .insert(VariableId::LoopValue, matched);
            let node = self.nodes.get_mut(best_index).unwrap();
            node.resume_with(model, controller)
        } else {
            UnpoweredFunctionState::Failed
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        self.compared_embedding = vec![];
        self.best_index = None;
    }
}
