use crate::prelude::*;

mod interrogate;
pub use interrogate::Interrogate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BarkWrapper {
    Interrogate(TextValue),
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
        }
    }
}
