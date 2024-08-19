mod messages;
use behavior_bark::unpowered::{UnpoweredFunction, UserNodeDefinition};
pub use messages::*;
mod prompting;
pub use prompting::*;
mod interrogate;
pub use interrogate::*;
mod wrappers;
use serde::{Deserialize, Serialize};
pub use wrappers::*;

use super::{BarkController, BarkModel, PromptValue, VariableId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BarkNode {
    ResetMessages(VariableId),
    AddUserMessage(VariableId, String),
    AddSystemMessage(VariableId, String),
    AddUserFromVariable(VariableId, VariableId),
    AddSystemFromVariable(VariableId, VariableId),
    Prompt(PromptValue),
    Revise(VariableId, PromptValue),
    RequireInResponse(Vec<String>, PromptValue),
    RejectInResponse(Vec<String>, PromptValue),
}

impl UserNodeDefinition for BarkNode {
    type Controller = BarkController;
    type Model = BarkModel;

    fn create_node(
        &self,
    ) -> Box<dyn UnpoweredFunction<Model = Self::Model, Controller = Self::Controller> + Send + Sync>
    {
        match self {
            BarkNode::ResetMessages(id) => Box::new(ResetMessages(id.clone())),
            BarkNode::AddUserMessage(id, message) => {
                Box::new(AddUserMessage(id.clone(), message.clone()))
            }
            BarkNode::AddSystemMessage(id, message) => {
                Box::new(AddSystemMessage(id.clone(), message.clone()))
            }
            BarkNode::AddUserFromVariable(id, variable) => {
                Box::new(AddUserFromVariable(id.clone(), variable.clone()))
            }
            BarkNode::AddSystemFromVariable(id, variable) => {
                Box::new(AddSystemFromVariable(id.clone(), variable.clone()))
            }
            BarkNode::Prompt(prompt) => Box::new(Prompt(prompt.clone())),
            BarkNode::Revise(id, prompt) => Box::new(Revise(id.clone(), prompt.clone())),
            BarkNode::RequireInResponse(words, prompt) => {
                Box::new(RequireInResponse(words.clone(), prompt.clone()))
            }
            BarkNode::RejectInResponse(words, prompt) => {
                Box::new(RejectInResponse(words.clone(), prompt.clone()))
            }
        }
    }
}
