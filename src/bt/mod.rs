mod nodes;
use behavior_bark::unpowered::{UnpoweredFunction, UnpoweredTreeDef};
pub use nodes::*;
mod controller;
mod model;
pub use controller::*;
pub use model::*;
pub mod values;

pub type BarkDef = UnpoweredTreeDef<BarkNode, BarkWrapper>;

pub type BarkFunction =
    Box<dyn UnpoweredFunction<Controller = BarkController, Model = BarkModel> + Send + Sync>;
