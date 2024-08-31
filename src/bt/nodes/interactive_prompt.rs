use std::io::Write;

use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InteractivePrompt(pub usize, pub PromptValue);

impl UnpoweredFunction for InteractivePrompt {
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
        let mut results = multi_prompt(self.0, &prompt, model);
        if results.is_empty() {
            UnpoweredFunctionState::Failed
        } else {
            loop {
                ask_for_input(&results);
                let input = model.read_stdin(true);
                if input.trim().eq_ignore_ascii_case("q") {
                    return UnpoweredFunctionState::Failed;
                } else if input.trim().eq_ignore_ascii_case("e") {
                    let input = model.read_stdin(true);
                    let mut new_prompt: Vec<Message> = prompt.clone();
                    let original_final_content = new_prompt.pop().unwrap().content;
                    new_prompt.push(user(&format!("{}\n{}", original_final_content, input)));
                    results = multi_prompt(self.0, &new_prompt, model);
                } else if input.trim().eq_ignore_ascii_case("r") {
                    results = multi_prompt(self.0, &prompt, model);
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
                    results = multi_prompt(3, &new_prompt, model);
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

fn multi_prompt(count: usize, prompt: &Vec<Message>, model: &BarkModel) -> Vec<String> {
    let mut results = vec![];
    for _ in 0..count {
        print!("."); // Progress indicator
        std::io::stdout().flush();
        let (output, result) = unpowered_prompt(prompt.clone(), model);
        if result == UnpoweredFunctionState::Complete {
            results.push(output);
        } else {
            break;
        }
    }
    println!();
    results
}

fn ask_for_input(results: &Vec<String>) {
    println!("Pick your favorite:");
    for (i, output) in results.iter().enumerate() {
        println!("{}: {}", i, output);
    }
    println!("q: Quit");
    println!("r: retry");
    println!("e: extend the original prompt");
    println!("x: Give a new prompt with the above as context.");
}
