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

use crate::{clients::ToolCaller, prelude::read_tree};

use super::{values::*, BarkController, BarkModel, BarkState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BarkNode<TC: ToolCaller> {
    Subtree(String),
    // Simple variable operations.
    SetText(VariableId, TextValue),
    SetTemplate(VariableId, Vec<MessageValue>),
    StartPrompt(VariableId, PromptValue),
    ExtendPrompt(VariableId, PromptValue),
    ReplaceSystemPrompt(VariableId, PromptValue),
    GetEmbedding(TextValue, VariableId),
    // Run prompts.
    Chat(Vec<MessageValue>),
    ChatWith(TextValue, Vec<MessageValue>),
    Prompt(PromptValue),
    PromptWith(TextValue, PromptValue),
    InteractivePrompt {
        choices: usize,
        chat: Vec<MessageValue>,
    },
    InteractivePromptWith {
        ai_model: Option<TextValue>,
        choices: usize,
        chat: Vec<MessageValue>,
    },
    // Agent (tool-use through MCP).
    Agent(PromptValue),
    AgentWithFilters {
        prompt: PromptValue,
        tool_filters: TextValue,
    },
    AgentWithFiltersAndModel {
        prompt: PromptValue,
        tool_filters: TextValue,
        ai_model: TextValue,
    },
    // Response checks
    MatchResponse(Option<TextValue>, TextMatcher, PromptValue),
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
    PushSimpleEmbedding(TextValue, TextValue),
    PushEmbeddingKeyValues(TextValue, TextValue, Vec<(TextValue, TextValue)>),
    PullBestScored(TextValue, TextValue),
    PullBestQueryMatch(TextValue, TextValue),
    Phantom(std::marker::PhantomData<TC>),
}

enum Subtree<TC: ToolCaller> {
    Uninitialized(String),
    Initialized(
        Box<dyn BehaviorTree<Model = BarkModel<TC>, Controller = BarkController> + Send + Sync>,
    ),
}

impl<TC: ToolCaller> BehaviorTree for Subtree<TC> {
    type Model = BarkModel<TC>;
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

impl<TC: ToolCaller> UserNodeDefinition for BarkNode<TC> {
    type Controller = BarkController;
    type Model = BarkModel<TC>;

    fn create_node(
        &self,
    ) -> Box<dyn BehaviorTree<Model = Self::Model, Controller = Self::Controller> + Send + Sync>
    {
        match self {
            BarkNode::Subtree(name) => Box::new(Subtree::<TC>::Uninitialized(name.clone())),
            BarkNode::SetText(id, text) => Box::new(SetText::<TC>(
                id.clone(),
                text.clone(),
                std::marker::PhantomData,
            )),
            BarkNode::SetTemplate(id, template) => Box::new(SetTemplate::<TC>(
                id.clone(),
                template.clone(),
                std::marker::PhantomData,
            )),
            BarkNode::StartPrompt(id, messages) => Box::new(StartPrompt::<TC>(
                id.clone(),
                messages.clone(),
                std::marker::PhantomData,
            )),
            BarkNode::ExtendPrompt(id, messages) => Box::new(ExtendPrompt::<TC>(
                id.clone(),
                messages.clone(),
                std::marker::PhantomData,
            )),
            BarkNode::ReplaceSystemPrompt(id, messages) => Box::new(ReplaceSystemPrompt::<TC>(
                id.clone(),
                messages.clone(),
                std::marker::PhantomData,
            )),
            BarkNode::Chat(messages) => Box::new(Prompt::<TC> {
                ai_model: None,
                prompt: PromptValue::Chat(messages.clone()),
                join_handle: None,
                prompt_id: None,
                _phantom: std::marker::PhantomData,
            }),
            BarkNode::ChatWith(model, messages) => Box::new(Prompt::<TC> {
                ai_model: Some(model.clone()),
                prompt: PromptValue::Chat(messages.clone()),
                join_handle: None,
                prompt_id: None,
                _phantom: std::marker::PhantomData,
            }),
            BarkNode::Prompt(prompt) => Box::new(Prompt::<TC> {
                ai_model: None,
                prompt: prompt.clone(),
                join_handle: None,
                prompt_id: None,
                _phantom: std::marker::PhantomData,
            }),
            BarkNode::PromptWith(model, prompt) => Box::new(Prompt::<TC> {
                ai_model: Some(model.clone()),
                prompt: prompt.clone(),
                join_handle: None,
                prompt_id: None,
                _phantom: std::marker::PhantomData,
            }),
            BarkNode::InteractivePrompt { choices, chat } => Box::new(InteractivePrompt::<TC> {
                ai_model: None,
                choices: *choices,
                prompt: PromptValue::Chat(chat.clone()),
                join_handle: None,
                _phantom: std::marker::PhantomData,
            }),
            BarkNode::InteractivePromptWith {
                ai_model,
                choices,
                chat,
            } => Box::new(InteractivePrompt::<TC> {
                ai_model: ai_model.clone(),
                choices: *choices,
                prompt: PromptValue::Chat(chat.clone()),
                join_handle: None,
                _phantom: std::marker::PhantomData,
            }),
            BarkNode::MatchResponse(ai_model, matches, prompt) => Box::new(MatchResponse::<TC> {
                ai_model: ai_model.clone(),
                matches: matches.clone(),
                prompt: prompt.clone(),
                join_handle: None,
                _phantom: std::marker::PhantomData,
            }),
            BarkNode::RequireInResponse(words, prompt) => Box::new(MatchResponse::<TC> {
                ai_model: None,
                matches: TextMatcher::Any(
                    words
                        .iter()
                        .map(|w| TextMatcher::Exact(TextValue::Simple(w.clone())))
                        .collect(),
                ),
                prompt: prompt.clone(),
                join_handle: None,
                _phantom: std::marker::PhantomData,
            }),
            BarkNode::RejectInResponse(words, prompt) => Box::new(MatchResponse::<TC> {
                ai_model: None,
                matches: TextMatcher::Not(Box::new(TextMatcher::Any(
                    words
                        .iter()
                        .map(|w| TextMatcher::Exact(TextValue::Simple(w.clone())))
                        .collect(),
                ))),
                prompt: prompt.clone(),
                join_handle: None,
                _phantom: std::marker::PhantomData,
            }),
            BarkNode::Agent(prompt) => Box::new(Agent::<TC> {
                ai_model: None,
                prompt: prompt.clone(),
                tool_filters: TextValue::Simple(String::new()), // Default to no filters
                join_handle: None,
                prompt_id: None,
                _phantom: std::marker::PhantomData,
            }),
            BarkNode::AgentWithFilters {
                prompt,
                tool_filters,
            } => Box::new(Agent::<TC> {
                ai_model: None,
                prompt: prompt.clone(),
                tool_filters: tool_filters.clone(),
                join_handle: None,
                prompt_id: None,
                _phantom: std::marker::PhantomData,
            }),
            BarkNode::AgentWithFiltersAndModel {
                prompt,
                tool_filters,
                ai_model,
            } => Box::new(Agent::<TC> {
                ai_model: Some(ai_model.clone()),
                prompt: prompt.clone(),
                tool_filters: tool_filters.clone(),
                join_handle: None,
                prompt_id: None,
                _phantom: std::marker::PhantomData,
            }),
            BarkNode::SaveFile { path, content } => Box::new(SaveFile::<TC> {
                path: path.clone(),
                content: content.clone(),
                _phantom: std::marker::PhantomData,
            }),
            BarkNode::SaveIndexedFile { path, content } => Box::new(SaveIndexedFile::<TC> {
                path: path.clone(),
                content: content.clone(),
                index: 0,
                _phantom: std::marker::PhantomData,
            }),
            BarkNode::LoadFile { path, content } => Box::new(LoadFile::<TC> {
                path: path.clone(),
                content: content.clone(),
                _phantom: std::marker::PhantomData,
            }),
            BarkNode::LoadIndexedFile { path, content } => Box::new(LoadIndexedFile::<TC> {
                path: path.clone(),
                content: content.clone(),
                index: 0,
                _phantom: std::marker::PhantomData,
            }),
            BarkNode::ReadLine(id) => {
                Box::new(ReadStdio::<TC>(true, id.clone(), std::marker::PhantomData))
            }
            BarkNode::ReadLines(id) => {
                Box::new(ReadStdio::<TC>(false, id.clone(), std::marker::PhantomData))
            }
            BarkNode::AskForInput(text) => {
                Box::new(AskForInput::<TC>(text.clone(), std::marker::PhantomData))
            }
            BarkNode::PrintLine(text) => {
                Box::new(PrintLine::<TC>(text.clone(), std::marker::PhantomData))
            }
            BarkNode::Unescape(id) => {
                Box::new(Unescape::<TC>(id.clone(), std::marker::PhantomData))
            }
            BarkNode::GetEmbedding(text, id) => Box::new(GetEmbedding::<TC> {
                text: text.clone(),
                variable: id.clone(),
                join_handle: None,
                _phantom: std::marker::PhantomData,
            }),
            BarkNode::PushSimpleEmbedding(path, text) => Box::new(PushSimpleEmbedding::<TC> {
                db: path.clone(),
                text: text.clone(),
                join_handle: None,
                _phantom: std::marker::PhantomData,
            }),
            BarkNode::PushEmbeddingKeyValues(path, text, values) => {
                Box::new(PushValuedEmbedding::<TC> {
                    db: path.clone(),
                    text: text.clone(),
                    kvs: values.clone(),
                    join_handle: None,
                    _phantom: std::marker::PhantomData,
                })
            }
            BarkNode::PullBestScored(path, text) => Box::new(PullBestScored::<TC> {
                db: path.clone(),
                text: text.clone(),
                join_handle: None,
                _phantom: std::marker::PhantomData,
            }),
            BarkNode::PullBestQueryMatch(path, text) => Box::new(PullBestScored::<TC> {
                db: path.clone(),
                text: TextValue::Multi(vec![
                    TextValue::Variable(VariableId::PreEmbed),
                    text.clone(),
                ]),
                join_handle: None,
                _phantom: std::marker::PhantomData,
            }),
            BarkNode::Phantom(_) => panic!("Phantom node should not be created"),
        }
    }
}
