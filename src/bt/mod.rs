mod nodes;
use behavior_bark::powered::{BehaviorTree, BehaviorTreeDef, BehaviorTreeState};
pub use nodes::*;
mod controller;
mod model;
pub use controller::*;
pub use model::*;

use crate::clients::ToolCaller;
pub mod values;

pub type BarkDef<TC> = BehaviorTreeDef<BarkNode<TC>, BarkWrapper<TC>>;

pub type BarkFunction<TC> =
    Box<dyn BehaviorTree<Controller = BarkController, Model = BarkModel<TC>> + Send + Sync>;

pub type BarkState = BehaviorTreeState;
