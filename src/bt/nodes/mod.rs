mod io;
mod messages;
use behavior_bark::unpowered::{UnpoweredFunction, UserNodeDefinition};
pub use io::*;
pub use messages::*;
mod prompting;
pub use prompting::*;
mod wrappers;
use serde::{Deserialize, Serialize};
pub use wrappers::*;
mod embedding;
pub use embedding::*;

use super::{values::*, BarkController, BarkModel};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BarkNode {
    // Modify prompts.
    StartPrompt(VariableId, Vec<MessageValue>),
    ExtendPrompt(VariableId, Vec<MessageValue>),
    // Run prompts.
    Chat(Vec<MessageValue>),
    Prompt(PromptValue),
    Revise(VariableId, PromptValue),
    // Response checks
    RequireInResponse(Vec<String>, PromptValue),
    RejectInResponse(Vec<String>, PromptValue),
    // STDIO
    ReadLine(VariableId),
    ReadLines(VariableId),
    PrintLine(TextValue),
    // Embeddings
    GetEmbedding(TextValue, VariableId),
}

impl UserNodeDefinition for BarkNode {
    type Controller = BarkController;
    type Model = BarkModel;

    fn create_node(
        &self,
    ) -> Box<dyn UnpoweredFunction<Model = Self::Model, Controller = Self::Controller> + Send + Sync>
    {
        match self {
            BarkNode::StartPrompt(id, messages) => {
                Box::new(StartPrompt(id.clone(), messages.clone()))
            }
            BarkNode::ExtendPrompt(id, messages) => {
                Box::new(ExtendPrompt(id.clone(), messages.clone()))
            }
            BarkNode::Chat(messages) => Box::new(Prompt(PromptValue::Chat(messages.clone()))),
            BarkNode::Prompt(prompt) => Box::new(Prompt(prompt.clone())),
            BarkNode::Revise(id, prompt) => Box::new(Revise(id.clone(), prompt.clone())),
            BarkNode::RequireInResponse(words, prompt) => {
                Box::new(RequireInResponse(words.clone(), prompt.clone()))
            }
            BarkNode::RejectInResponse(words, prompt) => {
                Box::new(RejectInResponse(words.clone(), prompt.clone()))
            }
            BarkNode::ReadLine(id) => Box::new(ReadStdio(true, id.clone())),
            BarkNode::ReadLines(id) => Box::new(ReadStdio(false, id.clone())),
            BarkNode::PrintLine(text) => Box::new(PrintLine(text.clone())),
            BarkNode::GetEmbedding(text, id) => Box::new(GetEmbedding(text.clone(), id.clone())),
        }
    }
}
