use std::hash::Hash;

use crate::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum VariableId {
    LoopValue,
    Accumulator,
    LastOutput,
    PreEmbed,
    User(String),
    PreLoaded(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PromptValue {
    Variable(VariableId),
    Template(VariableId),
    TemplateFile(TextValue),
    Quick(String),
    Chat(Vec<MessageValue>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextValue {
    Variable(VariableId),
    Thoughts(VariableId),
    WithoutThoughts(VariableId),
    Default(VariableId, String),
    Simple(String),
    Multi(Vec<TextValue>),
    Structured(HashMap<String, TextValue>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextMatcher {
    Exact(TextValue),
    Contains(TextValue),
    StartsWith(TextValue),
    EndsWith(TextValue),
    // Regex(TextValue),
    Not(Box<TextMatcher>),
    Any(Vec<TextMatcher>),
    All(Vec<TextMatcher>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageValue {
    User(String),
    System(String),
    Assistant(String),
    UserVar(VariableId),
    SystemVar(VariableId),
    AssistantVar(VariableId),
    UserVal(TextValue),
    SystemVal(TextValue),
    AssistantVal(TextValue),
    SubPrompt(VariableId),
    Template(VariableId),
}
