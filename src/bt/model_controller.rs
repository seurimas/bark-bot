use crate::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum VariableId {
    LoopValue,
    Accumulator,
    LastOutput,
    User(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PromptValue {
    Variable(VariableId),
    SimpleUserMessage(String),
    HardCoded(Vec<Message>),
}

#[derive(Default, Debug, Clone)]
pub struct BarkController {
    pub text_variables: HashMap<VariableId, String>,
    pub prompts: HashMap<VariableId, Vec<Message>>,
}

impl BarkController {
    pub fn get_prompt(&self, prompt: &PromptValue) -> Option<Vec<Message>> {
        match prompt {
            PromptValue::Variable(id) => self.prompts.get(id).cloned(),
            PromptValue::SimpleUserMessage(s) => Some(vec![user(s)]),
            PromptValue::HardCoded(messages) => Some(messages.clone()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BarkModel {
    pub client: OpenAI,
}

pub type BarkFunction =
    Box<dyn UnpoweredFunction<Controller = BarkController, Model = BarkModel> + Send + Sync>;
