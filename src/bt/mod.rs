mod nodes;
use behavior_bark::unpowered::UnpoweredTreeDef;
pub use nodes::*;
mod model_controller;
pub use model_controller::*;

pub type BarkDef = UnpoweredTreeDef<BarkNode, BarkWrapper>;
