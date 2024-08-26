use std::io::{self, Write};

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
pub struct PickBestPrompt(pub usize, pub PromptValue);

impl UnpoweredFunction for PickBestPrompt {
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
        let mut results = vec![];
        for _ in 0..self.0 {
            print!("."); // Progress indicator
            io::stdout().flush();
            let (output, result) = unpowered_prompt(prompt.clone(), model);
            if result == UnpoweredFunctionState::Complete {
                results.push(output);
            } else {
                break;
            }
        }
        println!();
        if results.is_empty() {
            UnpoweredFunctionState::Failed
        } else {
            println!("Pick your favorite:");
            for (i, output) in results.iter().enumerate() {
                println!("{}: {}", i, output);
            }
            println!("q: Quit");
            println!("x: Give prompt with the above as context.");
            loop {
                let input = model.read_stdin(true);
                if input.trim().eq_ignore_ascii_case("q") {
                    return UnpoweredFunctionState::Failed;
                } else if input.trim().eq_ignore_ascii_case("x") {
                    let input = model.read_stdin(true);
                    let new_messages: Vec<Message> = results
                        .iter()
                        .enumerate()
                        .map(|(i, s)| user(&format!("Item {}:\n{}", i, s)))
                        .collect::<Vec<Message>>();
                    let pre_prompt = vec![user(&"Use the following context to answer a new prompt. The context is composed of several items which might be referenced by the prompt. Context:\n")];
                    let mut new_prompt: Vec<Message> = pre_prompt
                        .iter()
                        .cloned()
                        .chain(new_messages.iter().cloned())
                        .collect();
                    new_prompt.push(user(&"\nPrompt:\n"));
                    new_prompt.push(user(&input));
                    results = vec![];
                    for _ in 0..self.0 {
                        print!("."); // Progress indicator
                        io::stdout().flush();
                        let (output, result) = unpowered_prompt(prompt.clone(), model);
                        if result == UnpoweredFunctionState::Complete {
                            results.push(output);
                        } else {
                            break;
                        }
                    }
                    for (i, output) in results.iter().enumerate() {
                        println!("{}: {}", i, output);
                    }
                } else if let Ok(index) = input.trim().parse::<usize>() {
                    if index < results.len() {
                        controller
                            .text_variables
                            .insert(VariableId::LastOutput, results[index].clone());
                        return UnpoweredFunctionState::Complete;
                    } else {
                        println!("Invalid index. Try again or q to quit.");
                    }
                } else {
                    println!("Invalid input. Try again or q to quit.");
                }
            }
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
