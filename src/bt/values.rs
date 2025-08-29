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

impl From<&str> for VariableId {
    fn from(value: &str) -> Self {
        VariableId::User(value.to_string())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PromptValue {
    Variable(VariableId),
    Template(VariableId),
    TemplateFile(TextValue),
    Quick(String),
    Chat(Vec<MessageValue>),
    Joined(Vec<PromptValue>),
}

impl From<Vec<MessageValue>> for PromptValue {
    fn from(messages: Vec<MessageValue>) -> Self {
        PromptValue::Chat(messages)
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum TextValue {
    Simple(String),
    Variable(VariableId),
    Thoughts(VariableId),
    WithoutThoughts(VariableId),
    Default(VariableId, String),
    Multi(Vec<TextValue>),
    Structured(HashMap<String, TextValue>),
}

impl<'de> Deserialize<'de> for TextValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        pub enum TextValueHelper {
            Variable(VariableId),
            Thoughts(VariableId),
            WithoutThoughts(VariableId),
            Default(VariableId, String),
            Simple(String),
            Multi(Vec<TextValue>),
            Structured(HashMap<String, TextValue>),
            #[serde(untagged)]
            Untagged(String),
        }

        let helper = TextValueHelper::deserialize(deserializer);
        match helper {
            Ok(value) => match value {
                TextValueHelper::Variable(v) => Ok(TextValue::Variable(v)),
                TextValueHelper::Thoughts(v) => Ok(TextValue::Thoughts(v)),
                TextValueHelper::WithoutThoughts(v) => Ok(TextValue::WithoutThoughts(v)),
                TextValueHelper::Default(v, d) => Ok(TextValue::Default(v, d)),
                TextValueHelper::Simple(s) => Ok(TextValue::Simple(s)),
                TextValueHelper::Multi(m) => Ok(TextValue::Multi(m)),
                TextValueHelper::Structured(s) => Ok(TextValue::Structured(s)),
                TextValueHelper::Untagged(s) => Ok(TextValue::Simple(s)),
            },
            Err(e) => Err(e),
        }
    }
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
