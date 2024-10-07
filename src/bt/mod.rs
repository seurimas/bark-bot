mod nodes;
use behavior_bark::powered::{BehaviorTree, BehaviorTreeDef, BehaviorTreeState};
pub use nodes::*;
mod controller;
mod model;
pub use controller::*;
pub use model::*;
pub mod values;

pub type BarkDef = BehaviorTreeDef<BarkNode, BarkWrapper>;

pub type BarkFunction =
    Box<dyn BehaviorTree<Controller = BarkController, Model = BarkModel> + Send + Sync>;

pub type BarkState = BehaviorTreeState;
