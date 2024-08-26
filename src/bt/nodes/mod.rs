mod db;
mod io;
mod messages;
use behavior_bark::unpowered::{UnpoweredFunction, UnpoweredFunctionState, UserNodeDefinition};
pub use db::*;
pub use io::*;
pub use messages::*;
mod prompting;
pub use prompting::*;
mod wrappers;
use serde::{Deserialize, Serialize};
pub use wrappers::*;
mod embedding;
pub use embedding::*;
mod search;
pub use search::*;

use crate::prelude::read_tree;

use super::{values::*, BarkController, BarkModel};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BarkNode {
    Subtree(String),
    // Simple variable operations.
    SetText(VariableId, TextValue),
    // Modify prompts.
    StartPrompt(VariableId, Vec<MessageValue>),
    ExtendPrompt(VariableId, Vec<MessageValue>),
    // Run prompts.
    Chat(Vec<MessageValue>),
    Prompt(PromptValue),
    PickBestPrompt(usize, PromptValue),
    // Response checks
    RequireInResponse(Vec<String>, PromptValue),
    RejectInResponse(Vec<String>, PromptValue),
    // Files
    SaveFile { path: TextValue, content: TextValue },
    // STDIO
    ReadLine(VariableId),
    ReadLines(VariableId),
    AskForInput(TextValue),
    PrintLine(TextValue),
    // Embeddings
    GetEmbedding(TextValue, VariableId),
    // Vector database
    PushSimpleEmbedding(String, TextValue),
    PushValuedEmbedding(String, TextValue, TextValue),
    PullBestScored(String, TextValue),
    PullBestQueryMatch(String, TextValue),
    // Search
    Search(TextValue),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetText(pub VariableId, pub TextValue);

impl UnpoweredFunction for SetText {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        let text = controller.get_text(&self.1);
        controller.text_variables.insert(self.0.clone(), text);
        UnpoweredFunctionState::Complete
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

impl UserNodeDefinition for BarkNode {
    type Controller = BarkController;
    type Model = BarkModel;

    fn create_node(
        &self,
    ) -> Box<dyn UnpoweredFunction<Model = Self::Model, Controller = Self::Controller> + Send + Sync>
    {
        match self {
            BarkNode::Subtree(name) => {
                let tree_def = read_tree(name);
                tree_def.create_tree()
            }
            BarkNode::SetText(id, text) => Box::new(SetText(id.clone(), text.clone())),
            BarkNode::StartPrompt(id, messages) => {
                Box::new(StartPrompt(id.clone(), messages.clone()))
            }
            BarkNode::ExtendPrompt(id, messages) => {
                Box::new(ExtendPrompt(id.clone(), messages.clone()))
            }
            BarkNode::Chat(messages) => Box::new(Prompt(PromptValue::Chat(messages.clone()))),
            BarkNode::Prompt(prompt) => Box::new(Prompt(prompt.clone())),
            BarkNode::PickBestPrompt(count, prompt) => {
                Box::new(PickBestPrompt(*count, prompt.clone()))
            }
            BarkNode::RequireInResponse(words, prompt) => {
                Box::new(RequireInResponse(words.clone(), prompt.clone()))
            }
            BarkNode::RejectInResponse(words, prompt) => {
                Box::new(RejectInResponse(words.clone(), prompt.clone()))
            }
            BarkNode::SaveFile { path, content } => Box::new(SaveFile {
                path: path.clone(),
                content: content.clone(),
            }),
            BarkNode::ReadLine(id) => Box::new(ReadStdio(true, id.clone())),
            BarkNode::ReadLines(id) => Box::new(ReadStdio(false, id.clone())),
            BarkNode::AskForInput(text) => Box::new(AskForInput(text.clone())),
            BarkNode::PrintLine(text) => Box::new(PrintLine(text.clone())),
            BarkNode::GetEmbedding(text, id) => Box::new(GetEmbedding(text.clone(), id.clone())),
            BarkNode::PushSimpleEmbedding(path, text) => {
                Box::new(PushSimpleEmbedding(path.clone(), text.clone()))
            }
            BarkNode::PushValuedEmbedding(path, text, values) => Box::new(PushValuedEmbedding(
                path.clone(),
                text.clone(),
                values.clone(),
            )),
            BarkNode::PullBestScored(path, text) => {
                Box::new(PullBestScored(path.clone(), text.clone()))
            }
            BarkNode::PullBestQueryMatch(path, text) => Box::new(PullBestScored(
                path.clone(),
                TextValue::Multi(vec![
                    TextValue::Variable(VariableId::PreEmbed),
                    text.clone(),
                ]),
            )),
            BarkNode::Search(text) => Box::new(Search(text.clone())),
        }
    }
}
