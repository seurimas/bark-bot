use crate::prelude::*;

mod branch_by_score;
// pub use branch_by_score::BranchByScore;
mod interrogate;
pub use interrogate::Interrogate;
mod knn;
pub use knn::Knn;
mod repl;
pub use repl::Repl;
mod repeat_until;
pub use repeat_until::RepeatUntil;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BarkWrapper<TC: ToolCaller> {
    Interrogate(TextValue),
    // BranchByScore(TextValue, Vec<TextValue>),
    Knn(String, TextValue, usize),
    KnnQuery(String, TextValue, usize),
    Repl(Option<TextValue>, Vec<TextValue>),
    RepeatUntil,
    Phantom(std::marker::PhantomData<TC>),
}

impl<TC: ToolCaller> UserWrapperDefinition<BarkNode<TC>> for BarkWrapper<TC> {
    fn create_node_and_wrap(
        &self,
        mut nodes: Vec<
            Box<dyn BehaviorTree<Model = BarkModel<TC>, Controller = BarkController> + Send + Sync>,
        >,
    ) -> Box<dyn BehaviorTree<Model = BarkModel<TC>, Controller = BarkController> + Send + Sync>
    {
        match self {
            BarkWrapper::Interrogate(text_value) => {
                Box::new(Interrogate::<TC>::new(text_value.clone(), nodes))
            }
            // BarkWrapper::BranchByScore(compared, options) => {
            //     Box::new(BranchByScore::new(compared.clone(), options.clone(), nodes))
            // }
            BarkWrapper::Knn(path, compared, k) => {
                Box::new(Knn::<TC>::new(path.clone(), compared.clone(), *k, nodes))
            }
            BarkWrapper::KnnQuery(path, compared, k) => Box::new(Knn::<TC>::new(
                path.clone(),
                TextValue::Multi(vec![
                    TextValue::Variable(VariableId::PreEmbed),
                    compared.clone(),
                ]),
                *k,
                nodes,
            )),
            BarkWrapper::Repl(prompt, options) => {
                Box::new(Repl::<TC>::new(prompt.clone(), options.clone(), nodes))
            }
            BarkWrapper::RepeatUntil => Box::new(RepeatUntil::<TC>::new(
                nodes.pop().unwrap(),
                nodes.pop().unwrap(),
            )),
            BarkWrapper::Phantom(_) => panic!("Phantom wrapper cannot be used to create a node"),
        }
    }
}
