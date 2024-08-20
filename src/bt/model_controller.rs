use openai_api_rust::Auth;

use crate::prelude::*;

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

    pub fn get_prompt(&self, prompt: &PromptValue) -> Vec<Message> {
        match prompt {
            PromptValue::Variable(id) => self.prompts.get(id).cloned().unwrap_or(vec![]),
            PromptValue::Quick(s) => vec![user(s)],
            PromptValue::Chat(messages) => {
                let mut chat = vec![];
                for message in messages {
                    match message {
                        MessageValue::User(s) => chat.push(user(s)),
                        MessageValue::System(s) => chat.push(system(s)),
                        MessageValue::UserVar(id) => chat.push(user(
                            &self.text_variables.get(id).cloned().unwrap_or_default(),
                        )),
                        MessageValue::SystemVar(id) => chat.push(system(
                            &self.text_variables.get(id).cloned().unwrap_or_default(),
                        )),
                        MessageValue::SubPrompt(id) => {
                            if let Some(sub_prompt) = self.prompts.get(id) {
                                chat.extend(sub_prompt.clone());
                            }
                        }
                    }
                }
                chat
            }
        }
    }

    pub fn get_text(&self, text: &TextValue) -> String {
        match text {
            TextValue::Variable(id) => self.text_variables.get(id).cloned().unwrap_or_default(),
            TextValue::Simple(s) => s.clone(),
        }
    }

    pub fn start_prompt(&mut self, id: VariableId, messages: Vec<MessageValue>) {
        let prompt = self.get_prompt(&PromptValue::Chat(messages));
        self.prompts.insert(id, prompt);
    }

    pub fn extend_prompt(&mut self, id: VariableId, messages: Vec<MessageValue>) {
        let prompt = self.get_prompt(&PromptValue::Chat(messages));
        self.prompts
            .entry(id)
            .or_insert_with(Vec::new)
            .extend(prompt);
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

    pub fn read_stdin(&self, line_only: bool) -> String {
        let mut text = String::new();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        if line_only {
            return line;
        }
        while !line.is_empty() {
            text.push_str(&line);
            line.clear();
            text.push('\n');
            std::io::stdin().read_line(&mut line).unwrap();
        }
        text
    }
}

pub type BarkFunction =
    Box<dyn UnpoweredFunction<Controller = BarkController, Model = BarkModel> + Send + Sync>;
