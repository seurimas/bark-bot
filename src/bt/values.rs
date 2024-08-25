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
    Simple(String),
    Multi(Vec<TextValue>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageValue {
    User(String),
    System(String),
    UserVar(VariableId),
    SystemVar(VariableId),
    SubPrompt(VariableId),
}
