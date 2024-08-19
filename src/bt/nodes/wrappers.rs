use crate::prelude::*;

use super::Interrogate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BarkWrapper {
    Interrogate,
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
            BarkWrapper::Interrogate => Box::new(Interrogate::new(nodes)),
        }
    }
}
