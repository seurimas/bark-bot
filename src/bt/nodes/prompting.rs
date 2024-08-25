use crate::prelude::*;

fn unpowered_prompt(prompt: Vec<Message>, model: &BarkModel) -> (String, UnpoweredFunctionState) {
    match model.chat_completion_create(&chat(prompt)) {
        Ok(mut response) => {
            if response.choices.is_empty() {
                eprintln!("Prompt Error (empty): {:?}", response);
                return ("".to_string(), UnpoweredFunctionState::Failed);
            } else if response.choices[0].message.is_none() {
                eprintln!("Prompt Error (empty message): {:?}", response);
                return ("".to_string(), UnpoweredFunctionState::Failed);
            } else if response.choices.len() > 1 {
                eprintln!("Prompt Warning (multiple choices): {:?}", response);
            }
            (
                response.choices.pop().unwrap().message.unwrap().content,
                UnpoweredFunctionState::Complete,
            )
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
        if prompt.is_empty() {
            return UnpoweredFunctionState::Failed;
        }
        let (output, result) = unpowered_prompt(prompt.clone(), model);
        if result == UnpoweredFunctionState::Complete {
            controller
                .text_variables
                .insert(VariableId::LastOutput, output);
        }
        result
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
        if prompt.is_empty() {
            return UnpoweredFunctionState::Failed;
        }
        let (output, result) = unpowered_prompt(prompt.clone(), model);
        if result == UnpoweredFunctionState::Complete {
            controller.text_variables.insert(self.0.clone(), output);
        }
        result
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
        if prompt.is_empty() {
            return UnpoweredFunctionState::Failed;
        }
        let (output, result) = unpowered_prompt(prompt.clone(), model);
        if result == UnpoweredFunctionState::Complete {
            controller
                .text_variables
                .insert(VariableId::LastOutput, output.clone());
            if self.0.iter().any(|s| output.to_lowercase().contains(s)) {
                UnpoweredFunctionState::Complete
            } else {
                UnpoweredFunctionState::Failed
            }
        } else {
            result
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
        if prompt.is_empty() {
            return UnpoweredFunctionState::Failed;
        }
        let (output, result) = unpowered_prompt(prompt.clone(), model);
        if result == UnpoweredFunctionState::Complete {
            controller
                .text_variables
                .insert(VariableId::LastOutput, output.clone());
            if self.0.iter().any(|s| output.to_lowercase().contains(s)) {
                UnpoweredFunctionState::Failed
            } else {
                UnpoweredFunctionState::Complete
            }
        } else {
            result
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
