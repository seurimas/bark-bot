use std::io::Write;

use crate::prelude::*;
use tokio::task::JoinHandle;

#[derive(Debug, Serialize, Deserialize)]
pub struct InteractivePrompt<TC: ToolCaller> {
    pub ai_model: Option<TextValue>,
    pub choices: usize,
    pub prompt: PromptValue,
    #[serde(skip)]
    pub join_handle: Option<JoinHandle<Vec<String>>>,
    #[serde(skip)]
    pub _phantom: std::marker::PhantomData<TC>,
}

impl<TC: ToolCaller> BehaviorTree for InteractivePrompt<TC> {
    type Controller = BarkController;
    type Model = BarkModel<TC>;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
        gas: &mut Option<i32>,
        mut audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        audit.enter(&"InteractivePrompt");
        let ai_model = self.ai_model.as_ref().map(|v| controller.get_text(v));
        if let Some(join_handle) = &mut self.join_handle {
            if let Ok(results) = try_join(join_handle) {
                self.join_handle = None;
                if results.is_empty() && self.join_handle.is_none() {
                    audit.mark(&"No results from multi_prompt");
                    audit.exit(&"InteractivePrompt", BarkState::Failed);
                    return BarkState::Failed;
                }
                self.join_handle = None;
                ask_for_input(&results);
                let input = model.read_stdin(true);
                if input.eq_ignore_ascii_case("q") {
                    audit.mark(&"User chose to quit");
                    audit.exit(&"InteractivePrompt", BarkState::Failed);
                    return BarkState::Failed;
                } else if input.eq_ignore_ascii_case("e") {
                    let input = model.read_stdin(true);
                    let mut new_prompt: Vec<BarkMessage> = controller.get_prompt(&self.prompt);
                    let original_final_content =
                        new_prompt.pop().unwrap().text_content().unwrap().clone();
                    new_prompt.push(user(&format!("{}\n{}", original_final_content, input)));
                    audit.mark(&"User extended the original prompt");
                    self.join_handle = Some(tokio::spawn(multi_prompt(
                        ai_model,
                        self.choices,
                        new_prompt,
                        model.clone(),
                        *gas,
                    )));
                    return BarkState::Waiting;
                } else if input.eq_ignore_ascii_case("r") {
                    audit.mark(&"User chose to retry the prompt");
                    self.join_handle = Some(tokio::spawn(multi_prompt(
                        ai_model,
                        self.choices,
                        controller.get_prompt(&self.prompt),
                        model.clone(),
                        *gas,
                    )));
                    return BarkState::Waiting;
                } else if input.eq_ignore_ascii_case("x") {
                    audit.mark(&"User chose to extend the prompt with context");
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
                    self.join_handle = Some(tokio::spawn(multi_prompt(
                        ai_model,
                        3,
                        new_prompt,
                        model.clone(),
                        *gas,
                    )));
                    return BarkState::Waiting;
                } else if let Ok(index) = input.parse::<usize>() {
                    if index < results.len() {
                        audit.mark(&format!("User selected index {}", index));
                        audit.exit(&"InteractivePrompt", BarkState::Complete);
                        controller
                            .text_variables
                            .insert(VariableId::LastOutput, results[index].clone());
                        return BarkState::Complete;
                    } else {
                        println!("Invalid index. Try again or q to quit.");
                        return BarkState::Failed; // TODO: FIX
                    }
                } else {
                    println!("Invalid input. Try again or q to quit.");
                    return BarkState::Failed; // TODO: FIX
                }
            } else {
                return BarkState::Waiting;
            }
        }
        self.join_handle = Some(tokio::spawn(multi_prompt(
            ai_model,
            self.choices,
            controller.get_prompt(&self.prompt),
            model.clone(),
            *gas,
        )));
        BarkState::Waiting
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

async fn multi_prompt<TC: ToolCaller>(
    ai_model: Option<String>,
    count: usize,
    prompt: Vec<BarkMessage>,
    model: BarkModel<TC>,
    mut gas: Option<i32>,
) -> Vec<String> {
    let mut results = vec![];
    for _ in 0..count {
        print!("."); // Progress indicator
        let _ = std::io::stdout().flush();
        let (output, result, new_gas) =
            powered_prompt(ai_model.clone(), prompt.clone(), model.clone(), gas).await;
        gas = new_gas; // TODO: handle gas properly
        if let Some(gas) = gas {
            if gas <= 0 {
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
