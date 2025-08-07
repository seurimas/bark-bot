mod db;
mod io;
mod mcp;
mod variables;
use behavior_bark::powered::{BehaviorTree, BehaviorTreeAudit, UserNodeDefinition};
pub use db::*;
pub use io::*;
use mcp::Agent;
pub use variables::*;
mod plain_prompt;
pub use plain_prompt::*;
mod wrappers;
use serde::{Deserialize, Serialize};
pub use wrappers::*;

use crate::prelude::read_tree;

use super::{values::*, BarkController, BarkModel, BarkState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BarkNode {
    Subtree(String),
    // Simple variable operations.
    SetText(VariableId, TextValue),
    SetTemplate(VariableId, Vec<MessageValue>),
    StartPrompt(VariableId, PromptValue),
    ExtendPrompt(VariableId, PromptValue),
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
    // Agent (tool-use through MCP).
    Agent(PromptValue),
    AgentWithFilters {
        prompt: PromptValue,
        tool_filters: Vec<String>,
    },
    AgentWithFiltersAndModel {
        prompt: PromptValue,
        tool_filters: Vec<String>,
        ai_model: String,
    },
    // Response checks
    MatchResponse(Option<String>, TextMatcher, PromptValue),
    RequireInResponse(Vec<String>, PromptValue),
    RejectInResponse(Vec<String>, PromptValue),
    // Files
    SaveFile {
        path: TextValue,
        content: TextValue,
    },
    SaveIndexedFile {
        path: TextValue,
        content: TextValue,
    },
    LoadFile {
        path: TextValue,
        content: VariableId,
    },
    LoadIndexedFile {
        path: TextValue,
        content: VariableId,
    },
    // STDIO
    ReadLine(VariableId),
    ReadLines(VariableId),
    AskForInput(TextValue),
    PrintLine(TextValue),
    Unescape(VariableId),
    // Vector database
    PushSimpleEmbedding(String, TextValue),
    PushEmbeddingKeyValues(String, TextValue, Vec<(TextValue, TextValue)>),
    PullBestScored(String, TextValue),
    PullBestQueryMatch(String, TextValue),
}

enum Subtree {
    Uninitialized(String),
    Initialized(
        Box<dyn BehaviorTree<Model = BarkModel, Controller = BarkController> + Send + Sync>,
    ),
}

impl BehaviorTree for Subtree {
    type Model = BarkModel;
    type Controller = BarkController;

    fn resume_with(
        &mut self,
        model: &Self::Model,
        controller: &mut Self::Controller,
        gas: &mut Option<i32>,
        audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        match self {
            Subtree::Uninitialized(name) => {
                let tree = read_tree(&model.tree_root, name);
                *self = Subtree::Initialized(tree.create_tree());
                self.resume_with(model, controller, gas, audit)
            }
            Subtree::Initialized(tree) => tree.resume_with(model, controller, gas, audit),
        }
    }

    fn reset(self: &mut Self, model: &Self::Model) {
        match self {
            Subtree::Uninitialized(_) => {}
            Subtree::Initialized(tree) => tree.reset(model),
        }
    }
}

impl UserNodeDefinition for BarkNode {
    type Controller = BarkController;
    type Model = BarkModel;

    fn create_node(
        &self,
    ) -> Box<dyn BehaviorTree<Model = Self::Model, Controller = Self::Controller> + Send + Sync>
    {
        match self {
            BarkNode::Subtree(name) => Box::new(Subtree::Uninitialized(name.clone())),
            BarkNode::SetText(id, text) => Box::new(SetText(id.clone(), text.clone())),
            BarkNode::SetTemplate(id, template) => {
                Box::new(SetTemplate(id.clone(), template.clone()))
            }
            BarkNode::StartPrompt(id, messages) => {
                Box::new(StartPrompt(id.clone(), messages.clone()))
            }
            BarkNode::ExtendPrompt(id, messages) => {
                Box::new(ExtendPrompt(id.clone(), messages.clone()))
            }
            BarkNode::Chat(messages) => Box::new(Prompt {
                ai_model: None,
                prompt: PromptValue::Chat(messages.clone()),
                join_handle: None,
                prompt_id: None,
            }),
            BarkNode::ChatWith(model, messages) => Box::new(Prompt {
                ai_model: Some(model.clone()),
                prompt: PromptValue::Chat(messages.clone()),
                join_handle: None,
                prompt_id: None,
            }),
            BarkNode::Prompt(prompt) => Box::new(Prompt {
                ai_model: None,
                prompt: prompt.clone(),
                join_handle: None,
                prompt_id: None,
            }),
            BarkNode::PromptWith(model, prompt) => Box::new(Prompt {
                ai_model: Some(model.clone()),
                prompt: prompt.clone(),
                join_handle: None,
                prompt_id: None,
            }),
            BarkNode::InteractivePrompt { choices, chat } => Box::new(InteractivePrompt {
                ai_model: None,
                choices: *choices,
                prompt: PromptValue::Chat(chat.clone()),
                join_handle: None,
            }),
            BarkNode::InteractivePromptWith {
                ai_model,
                choices,
                chat,
            } => Box::new(InteractivePrompt {
                ai_model: ai_model.clone(),
                choices: *choices,
                prompt: PromptValue::Chat(chat.clone()),
                join_handle: None,
            }),
            BarkNode::MatchResponse(ai_model, matches, prompt) => Box::new(MatchResponse {
                ai_model: ai_model.clone(),
                matches: matches.clone(),
                prompt: prompt.clone(),
                join_handle: None,
            }),
            BarkNode::RequireInResponse(words, prompt) => Box::new(MatchResponse {
                ai_model: None,
                matches: TextMatcher::Any(
                    words
                        .iter()
                        .map(|w| TextMatcher::Exact(TextValue::Simple(w.clone())))
                        .collect(),
                ),
                prompt: prompt.clone(),
                join_handle: None,
            }),
            BarkNode::RejectInResponse(words, prompt) => Box::new(MatchResponse {
                ai_model: None,
                matches: TextMatcher::Not(Box::new(TextMatcher::Any(
                    words
                        .iter()
                        .map(|w| TextMatcher::Exact(TextValue::Simple(w.clone())))
                        .collect(),
                ))),
                prompt: prompt.clone(),
                join_handle: None,
            }),
            BarkNode::Agent(prompt) => Box::new(Agent {
                ai_model: None,
                prompt: prompt.clone(),
                tool_filters: vec![],
                join_handle: None,
            }),
            BarkNode::AgentWithFilters {
                prompt,
                tool_filters,
            } => Box::new(Agent {
                ai_model: None,
                prompt: prompt.clone(),
                tool_filters: tool_filters.clone(),
                join_handle: None,
            }),
            BarkNode::AgentWithFiltersAndModel {
                prompt,
                tool_filters,
                ai_model,
            } => Box::new(Agent {
                ai_model: Some(ai_model.clone()),
                prompt: prompt.clone(),
                tool_filters: tool_filters.clone(),
                join_handle: None,
            }),
            BarkNode::SaveFile { path, content } => Box::new(SaveFile {
                path: path.clone(),
                content: content.clone(),
            }),
            BarkNode::SaveIndexedFile { path, content } => Box::new(SaveIndexedFile {
                path: path.clone(),
                content: content.clone(),
                index: 0,
            }),
            BarkNode::LoadFile { path, content } => Box::new(LoadFile {
                path: path.clone(),
                content: content.clone(),
            }),
            BarkNode::LoadIndexedFile { path, content } => Box::new(LoadIndexedFile {
                path: path.clone(),
                content: content.clone(),
                index: 0,
            }),
            BarkNode::ReadLine(id) => Box::new(ReadStdio(true, id.clone())),
            BarkNode::ReadLines(id) => Box::new(ReadStdio(false, id.clone())),
            BarkNode::AskForInput(text) => Box::new(AskForInput(text.clone())),
            BarkNode::PrintLine(text) => Box::new(PrintLine(text.clone())),
            BarkNode::Unescape(id) => Box::new(Unescape(id.clone())),
            BarkNode::GetEmbedding(text, id) => Box::new(GetEmbedding {
                text: text.clone(),
                variable: id.clone(),
                join_handle: None,
            }),
            BarkNode::PushSimpleEmbedding(path, text) => Box::new(PushSimpleEmbedding {
                db: path.clone(),
                text: text.clone(),
                join_handle: None,
            }),
            BarkNode::PushEmbeddingKeyValues(path, text, values) => Box::new(PushValuedEmbedding {
                db: path.clone(),
                text: text.clone(),
                kvs: values.clone(),
                join_handle: None,
            }),
            BarkNode::PullBestScored(path, text) => Box::new(PullBestScored {
                db: path.clone(),
                text: text.clone(),
                join_handle: None,
            }),
            BarkNode::PullBestQueryMatch(path, text) => Box::new(PullBestScored {
                db: path.clone(),
                text: TextValue::Multi(vec![
                    TextValue::Variable(VariableId::PreEmbed),
                    text.clone(),
                ]),
                join_handle: None,
            }),
        }
    }
}
