use std::hash::Hash;

use crate::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum VariableId {
    LoopValue,
    Accumulator,
    LastOutput,
    PreEmbed,
    User(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PromptValue {
    Variable(VariableId),
    Quick(String),
    Chat(Vec<MessageValue>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextValue {
    Variable(VariableId),
    Thoughts(VariableId),
    WithoutThoughts(VariableId),
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
    UserVar(VariableId),
    SystemVar(VariableId),
    UserVal(TextValue),
    SystemVal(TextValue),
    SubPrompt(VariableId),
}
