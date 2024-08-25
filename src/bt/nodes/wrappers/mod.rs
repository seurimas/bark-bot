use crate::prelude::*;

mod best_match;
pub use best_match::BestMatch;
mod interrogate;
pub use interrogate::Interrogate;
mod knn;
pub use knn::Knn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BarkWrapper {
    Interrogate(TextValue),
    BestMatch(TextValue, Vec<TextValue>),
    Knn(String, TextValue, usize),
    KnnQuery(String, TextValue, usize),
}

impl UserWrapperDefinition<BarkNode> for BarkWrapper {
    fn create_node_and_wrap(
        &self,
        nodes: Vec<
            Box<
                dyn UnpoweredFunction<Model = BarkModel, Controller = BarkController> + Send + Sync,
            >,
        >,
    ) -> Box<dyn UnpoweredFunction<Model = BarkModel, Controller = BarkController> + Send + Sync>
    {
        match self {
            BarkWrapper::Interrogate(text_value) => {
                Box::new(Interrogate::new(text_value.clone(), nodes))
            }
            BarkWrapper::BestMatch(compared, options) => {
                Box::new(BestMatch::new(compared.clone(), options.clone(), nodes))
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
