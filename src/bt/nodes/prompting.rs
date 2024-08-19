use openai_api_rust::chat::ChatApi;

use crate::prelude::*;

fn unpowered_prompt(prompt: Vec<Message>, model: &BarkModel) -> (String, UnpoweredFunctionState) {
    match model.client.chat_completion_create(&chat(prompt)) {
        Ok(response) => {
            let message = &response.choices[0].message;
            if let Some(message) = message {
                // XXX: Why do I need to clone here?
                (
                    message.content.to_string(),
                    UnpoweredFunctionState::Complete,
                )
            } else {
                eprintln!("Prompt Error (chat): {:?}", response);
                ("".to_string(), UnpoweredFunctionState::Failed)
            }
        }
        Err(e) => {
            eprintln!("Prompt Error: {:?}", e);
            ("".to_string(), UnpoweredFunctionState::Failed)
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prompt(pub PromptValue);

impl UnpoweredFunction for Prompt {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        let prompt = controller.get_prompt(&self.0);
        if let Some(prompt) = prompt {
            let (output, result) = unpowered_prompt(prompt.clone(), model);
            if result == UnpoweredFunctionState::Complete {
                controller
                    .text_variables
                    .insert(VariableId::LastOutput, output);
            }
            result
        } else {
            eprintln!("Error: No prompt found for {:?}", self.0);
            UnpoweredFunctionState::Failed
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Revise(pub VariableId, pub PromptValue);

impl UnpoweredFunction for Revise {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        let prompt = controller.get_prompt(&self.1);
        if let Some(prompt) = prompt {
            let (output, result) = unpowered_prompt(prompt.clone(), model);
            if result == UnpoweredFunctionState::Complete {
                controller.text_variables.insert(self.0.clone(), output);
            }
            result
        } else {
            eprintln!("Error: No prompt found for {:?}", self.1);
            UnpoweredFunctionState::Failed
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequireInResponse(pub Vec<String>, pub PromptValue);

impl UnpoweredFunction for RequireInResponse {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        let prompt = controller.get_prompt(&self.1);
        if let Some(prompt) = prompt {
            let (output, result) = unpowered_prompt(prompt.clone(), model);
            if result == UnpoweredFunctionState::Complete {
                if self.0.iter().any(|s| output.to_lowercase().contains(s)) {
                    UnpoweredFunctionState::Complete
                } else {
                    UnpoweredFunctionState::Failed
                }
            } else {
                result
            }
        } else {
            eprintln!("Error: No prompt found for {:?}", self.1);
            UnpoweredFunctionState::Failed
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RejectInResponse(pub Vec<String>, pub PromptValue);

impl UnpoweredFunction for RejectInResponse {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        let prompt = controller.get_prompt(&self.1);
        if let Some(prompt) = prompt {
            let (output, result) = unpowered_prompt(prompt.clone(), model);
            if result == UnpoweredFunctionState::Complete {
                if self.0.iter().any(|s| output.to_lowercase().contains(s)) {
                    UnpoweredFunctionState::Failed
                } else {
                    UnpoweredFunctionState::Complete
                }
            } else {
                result
            }
        } else {
            eprintln!("Error: No prompt found for {:?}", self.0);
            UnpoweredFunctionState::Failed
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
