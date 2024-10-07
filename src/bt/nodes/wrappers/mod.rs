use crate::prelude::*;

mod branch_by_score;
pub use branch_by_score::BranchByScore;
mod interrogate;
pub use interrogate::Interrogate;
mod knn;
pub use knn::Knn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BarkWrapper {
    Interrogate(TextValue),
    BranchByScore(TextValue, Vec<TextValue>),
    Knn(String, TextValue, usize),
    KnnQuery(String, TextValue, usize),
}

impl UserWrapperDefinition<BarkNode> for BarkWrapper {
    fn create_node_and_wrap(
        &self,
        nodes: Vec<
            Box<dyn BehaviorTree<Model = BarkModel, Controller = BarkController> + Send + Sync>,
        >,
    ) -> Box<dyn BehaviorTree<Model = BarkModel, Controller = BarkController> + Send + Sync> {
        match self {
            BarkWrapper::Interrogate(text_value) => {
                Box::new(Interrogate::new(text_value.clone(), nodes))
            }
            BarkWrapper::BranchByScore(compared, options) => {
                Box::new(BranchByScore::new(compared.clone(), options.clone(), nodes))
            }
            BarkWrapper::Knn(path, compared, k) => {
                Box::new(Knn::new(path.clone(), compared.clone(), *k, nodes))
            }
            BarkWrapper::KnnQuery(path, compared, k) => Box::new(Knn::new(
                path.clone(),
                TextValue::Multi(vec![
                    TextValue::Variable(VariableId::PreEmbed),
                    compared.clone(),
                ]),
                *k,
                nodes,
            )),
        }
    }
}
