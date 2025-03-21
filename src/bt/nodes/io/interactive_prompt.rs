use std::io::Write;

use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InteractivePrompt {
    pub ai_model: Option<String>,
    pub choices: usize,
    pub prompt: PromptValue,
}

impl BehaviorTree for InteractivePrompt {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
        gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        let prompt = controller.get_prompt(&self.prompt);
        if prompt.is_empty() {
            return BarkState::Failed;
        }
        let mut results = multi_prompt(self.ai_model.as_ref(), self.choices, &prompt, model, gas);
        check_gas!(gas);
        if results.is_empty() {
            BarkState::Failed
        } else {
            loop {
                check_gas!(gas);
                ask_for_input(&results);
                let input = model.read_stdin(true);
                if input.eq_ignore_ascii_case("q") {
                    return BarkState::Failed;
                } else if input.eq_ignore_ascii_case("e") {
                    let input = model.read_stdin(true);
                    let mut new_prompt: Vec<BarkMessage> = prompt.clone();
                    let original_final_content =
                        new_prompt.pop().unwrap().text_content().unwrap().clone();
                    new_prompt.push(user(&format!("{}\n{}", original_final_content, input)));
                    results = multi_prompt(
                        self.ai_model.as_ref(),
                        self.choices,
                        &new_prompt,
                        model,
                        gas,
                    );
                } else if input.eq_ignore_ascii_case("r") {
                    results =
                        multi_prompt(self.ai_model.as_ref(), self.choices, &prompt, model, gas);
                } else if input.eq_ignore_ascii_case("x") {
                    let input = model.read_stdin(true);
                    let new_messages: Vec<BarkMessage> = results
                        .iter()
                        .enumerate()
                        .map(|(i, s)| user(&format!("Item {}:\n{}", i, s)))
                        .collect::<Vec<BarkMessage>>();
                    let pre_prompt = vec![user(&"Use the following context to answer a new prompt. The context is composed of several items which might be referenced by the prompt. Context:\n")];
                    let mut new_prompt: Vec<BarkMessage> = pre_prompt
                        .iter()
                        .cloned()
                        .chain(new_messages.iter().cloned())
                        .collect();
                    new_prompt.push(user(&"\nPrompt:\n"));
                    new_prompt.push(user(&input));
                    results = multi_prompt(self.ai_model.as_ref(), 3, &new_prompt, model, gas);
                } else if let Ok(index) = input.parse::<usize>() {
                    if index < results.len() {
                        controller
                            .text_variables
                            .insert(VariableId::LastOutput, results[index].clone());
                        return BarkState::Complete;
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

fn multi_prompt(
    ai_model: Option<&String>,
    count: usize,
    prompt: &Vec<BarkMessage>,
    model: &BarkModel,
    gas: &mut Option<i32>,
) -> Vec<String> {
    let mut results = vec![];
    for _ in 0..count {
        print!("."); // Progress indicator
        let _ = std::io::stdout().flush();
        let (output, result) = powered_prompt(ai_model, prompt.clone(), model, gas);
        if let Some(gas) = gas {
            if *gas <= 0 {
                break;
            }
        }
        if result == BarkState::Complete {
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
