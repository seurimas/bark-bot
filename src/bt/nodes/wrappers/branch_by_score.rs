use crate::prelude::*;
use tokio::task::{block_in_place, JoinHandle};

// pub struct BranchByScore {
//     compared: TextValue,
//     compared_embedding: Vec<f32>,
//     text_values: Vec<TextValue>,
//     best_index: Option<usize>,
//     nodes: Vec<Box<dyn BehaviorTree<Model = BarkModel, Controller = BarkController> + Send + Sync>>,
//     join_handle: Option<JoinHandle<Result<(Vec<f32>, Option<i32>), String>>>,
// }

// impl BranchByScore {
//     pub fn new(
//         compared: TextValue,
//         text_values: Vec<TextValue>,
//         nodes: Vec<
//             Box<dyn BehaviorTree<Model = BarkModel, Controller = BarkController> + Send + Sync>,
//         >,
//     ) -> Self {
//         if nodes.len() != text_values.len() {
//             panic!("BranchByScore nodes and text_values must have the same length");
//         }
//         Self {
//             compared,
//             compared_embedding: vec![],
//             text_values,
//             best_index: None,
//             nodes,
//             join_handle: None,
//         }
//     }
// }

// impl BehaviorTree for BranchByScore {
//     type Controller = BarkController;
//     type Model = BarkModel;

//     fn resume_with(
//         self: &mut Self,
//         model: &Self::Model,
//         controller: &mut Self::Controller,
//         gas: &mut Option<i32>,
//         mut _audit: &mut Option<BehaviorTreeAudit>,
//     ) -> BarkState {
//         if self.compared_embedding.is_empty() && self.join_handle.is_none() {
//             let compared_text = controller.get_text(&self.compared);
//             self.join_handle = Some(tokio::spawn(model.get_embedding(&compared_text, gas)));
//             return BarkState::Waiting;
//         } else if let Some(join_handle) = &mut self.join_handle {
//             if let Ok(result) = try_join(join_handle) {
//                 let compared_embedding = result.0;
//                 *gas = result.1;
//                 check_gas!(gas);
//                 if let Ok(embedding) = embedding {
//                     self.compared_embedding = embedding;
//                 } else {
//                     return BarkState::Failed;
//                 }
//             } else {
//                 return BarkState::Waiting;
//             }
//         }
//         if self.best_index.is_none() {
//             let mut best_score = f32::INFINITY;
//             let mut best_index = 0;
//             for (index, text_value) in self.text_values.iter().enumerate() {
//                 let text_value = controller.get_text(text_value);
//                 let score = block_in_place(model.get_embedding(&text_value, gas))
//                     .map(|(embedding, _)| score(&self.compared_embedding, &embedding));
//                 check_gas!(gas);
//                 if let Ok(score) = score {
//                     if score < best_score {
//                         best_score = score;
//                         best_index = index;
//                     }
//                 }
//             }
//             self.best_index = Some(best_index);
//         }
//         if let Some(best_index) = self.best_index {
//             let matched = controller.get_text(&self.text_values[best_index]);
//             controller
//                 .text_variables
//                 .insert(VariableId::LoopValue, matched);
//             let node = self.nodes.get_mut(best_index).unwrap();
//             node.resume_with(model, controller, gas, _audit)
//         } else {
//             BarkState::Failed
//         }
//     }

//     fn reset(self: &mut Self, _model: &Self::Model) {
//         self.compared_embedding = vec![];
//         self.best_index = None;
//     }
// }
