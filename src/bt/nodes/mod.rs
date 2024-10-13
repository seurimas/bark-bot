mod db;
mod io;
mod variables;
use behavior_bark::powered::{BehaviorTree, UserNodeDefinition};
pub use db::*;
pub use io::*;
pub use variables::*;
mod plain_prompt;
pub use plain_prompt::*;
mod wrappers;
use serde::{Deserialize, Serialize};
pub use wrappers::*;
mod extensions;
pub use extensions::*;

use crate::prelude::read_tree;

use super::{values::*, BarkController, BarkModel};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BarkNode {
    Subtree(String),
    // Simple variable operations.
    SetText(VariableId, TextValue),
    StartPrompt(VariableId, Vec<MessageValue>),
    ExtendPrompt(VariableId, Vec<MessageValue>),
    GetEmbedding(TextValue, VariableId),
    // Run prompts.
    Chat(Vec<MessageValue>),
    ChatWith(String, Vec<MessageValue>),
    Prompt(PromptValue),
    PromptWith(String, PromptValue),
    InteractivePrompt {
        choices: usize,
        chat: Vec<MessageValue>,
    },
    InteractivePromptWith {
        ai_model: Option<String>,
        choices: usize,
        chat: Vec<MessageValue>,
    },
    // Response checks
    RequireInResponse(Vec<String>, PromptValue),
    RejectInResponse(Vec<String>, PromptValue),
    // Files
    SaveFile {
        path: TextValue,
        content: TextValue,
    },
    LoadFile {
        path: TextValue,
        content: VariableId,
    },
    // STDIO
    ReadLine(VariableId),
    ReadLines(VariableId),
    AskForInput(TextValue),
    PrintLine(TextValue),
    // Vector database
    PushSimpleEmbedding(String, TextValue),
    PushEmbeddingKeyValues(String, TextValue, Vec<(TextValue, TextValue)>),
    PullBestScored(String, TextValue),
    PullBestQueryMatch(String, TextValue),
    // Search
    Search(TextValue),
}

impl UserNodeDefinition for BarkNode {
    type Controller = BarkController;
    type Model = BarkModel;

    fn create_node(
        &self,
    ) -> Box<dyn BehaviorTree<Model = Self::Model, Controller = Self::Controller> + Send + Sync>
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
            BarkNode::Chat(messages) => Box::new(Prompt {
                ai_model: None,
                prompt: PromptValue::Chat(messages.clone()),
            }),
            BarkNode::ChatWith(model, messages) => Box::new(Prompt {
                ai_model: Some(model.clone()),
                prompt: PromptValue::Chat(messages.clone()),
            }),
            BarkNode::Prompt(prompt) => Box::new(Prompt {
                ai_model: None,
                prompt: prompt.clone(),
            }),
            BarkNode::PromptWith(model, prompt) => Box::new(Prompt {
                ai_model: Some(model.clone()),
                prompt: prompt.clone(),
            }),
            BarkNode::InteractivePrompt { choices, chat } => Box::new(InteractivePrompt {
                ai_model: None,
                choices: *choices,
                prompt: PromptValue::Chat(chat.clone()),
            }),
            BarkNode::InteractivePromptWith {
                ai_model,
                choices,
                chat,
            } => Box::new(InteractivePrompt {
                ai_model: ai_model.clone(),
                choices: *choices,
                prompt: PromptValue::Chat(chat.clone()),
            }),
            BarkNode::RequireInResponse(words, prompt) => Box::new(RequireInResponse {
                ai_model: None,
                matches: words.clone(),
                prompt: prompt.clone(),
            }),
            BarkNode::RejectInResponse(words, prompt) => Box::new(RejectInResponse {
                ai_model: None,
                matches: words.clone(),
                prompt: prompt.clone(),
            }),
            BarkNode::SaveFile { path, content } => Box::new(SaveFile {
                path: path.clone(),
                content: content.clone(),
            }),
            BarkNode::LoadFile { path, content } => Box::new(LoadFile {
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
            BarkNode::PushEmbeddingKeyValues(path, text, values) => Box::new(PushValuedEmbedding(
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
