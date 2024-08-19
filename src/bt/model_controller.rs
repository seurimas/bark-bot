use openai_api_rust::Auth;

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
    Chat(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextValue {
    Variable(VariableId),
    Simple(String),
}

#[derive(Default, Debug, Clone)]
pub struct BarkController {
    pub text_variables: HashMap<VariableId, String>,
    pub prompts: HashMap<VariableId, Vec<Message>>,
}

impl BarkController {
    pub fn new() -> Self {
        Self {
            text_variables: HashMap::new(),
            prompts: HashMap::new(),
        }
    }

    pub fn get_prompt(&self, prompt: &PromptValue) -> Option<Vec<Message>> {
        match prompt {
            PromptValue::Variable(id) => self.prompts.get(id).cloned(),
            PromptValue::SimpleUserMessage(s) => Some(vec![user(s)]),
            PromptValue::Chat(messages) => Some(
                messages
                    .iter()
                    .enumerate()
                    .map(|(i, m)| if i % 2 == 0 { user(m) } else { system(m) })
                    .collect(),
            ),
        }
    }

    pub fn get_text(&self, text: &TextValue) -> String {
        match text {
            TextValue::Variable(id) => self.text_variables.get(id).cloned().unwrap_or_default(),
            TextValue::Simple(s) => s.clone(),
        }
    }

    pub fn add_user_to_prompt(&mut self, id: VariableId, text: TextValue) {
        let text = self.get_text(&text);
        let messages = self.prompts.entry(id).or_insert_with(Vec::new);
        if messages.is_empty()
            || matches!(messages.last().unwrap().role, openai_api_rust::Role::System)
        {
            messages.push(user(&text));
        } else {
            messages.last_mut().unwrap().content.push_str(&text);
        }
    }

    pub fn add_system_to_prompt(&mut self, id: VariableId, text: TextValue) {
        let text = self.get_text(&text);
        let messages = self.prompts.entry(id).or_insert_with(Vec::new);
        if messages.is_empty()
            || matches!(messages.last().unwrap().role, openai_api_rust::Role::User)
        {
            messages.push(system(&text));
        } else {
            messages.last_mut().unwrap().content.push_str(&text);
        }
    }
}

#[derive(Debug, Clone)]
pub struct BarkModel {
    pub client: OpenAI,
}

impl BarkModel {
    pub fn new() -> Self {
        let auth = Auth::from_env().unwrap();
        let client = OpenAI::new(auth, &std::env::var("OPENAI_URL").unwrap());
        Self { client }
    }
}

pub type BarkFunction =
    Box<dyn UnpoweredFunction<Controller = BarkController, Model = BarkModel> + Send + Sync>;
