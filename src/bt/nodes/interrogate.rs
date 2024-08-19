use crate::prelude::*;

pub struct Interrogate {
    state: InterrogateState,
    current: String,
    remaining: String,
    wrapped: BarkFunction,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum InterrogateState {
    Uninitialized,
    Waited,
    NotWaited,
}

impl Interrogate {
    pub fn new(mut wrapped: Vec<BarkFunction>) -> Self {
        Self {
            state: InterrogateState::Uninitialized,
            current: "".to_string(),
            remaining: "".to_string(),
            wrapped: wrapped.pop().unwrap(),
        }
    }
}

impl UnpoweredFunction for Interrogate {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        if self.state == InterrogateState::Uninitialized {
            let output = controller.text_variables.get(&VariableId::LastOutput);
            match output {
                Some(output) => {
                    self.current = String::new();
                    self.remaining = output.clone();
                    self.state = InterrogateState::NotWaited;
                }
                None => {
                    eprintln!("Error: No output found for {:?}", VariableId::LastOutput);
                    return UnpoweredFunctionState::Failed;
                }
            }
        }
        while self.remaining.len() > 0 {
            match self.state {
                InterrogateState::NotWaited => {
                    // The node has not waited, so it has completed for the previous output.
                    let newline = self.remaining.find('\n');
                    match newline {
                        Some(index) => {
                            self.current = self.remaining[..index].to_string();
                            self.remaining = self.remaining[index + 1..].to_string();
                        }
                        None => {
                            self.current.push_str(&self.remaining);
                            self.remaining = "".to_string();
                        }
                    }
                }
                _ => {}
            }
            controller
                .text_variables
                .insert(VariableId::LoopValue, self.current.clone());
            let result = self.wrapped.resume_with(model, controller);
            match result {
                UnpoweredFunctionState::Complete => {
                    self.state = InterrogateState::NotWaited;
                }
                UnpoweredFunctionState::Waiting => {
                    self.state = InterrogateState::Waited;
                    return UnpoweredFunctionState::Waiting;
                }
                UnpoweredFunctionState::Failed => {
                    // XXX: Do we need to reset here?
                    return UnpoweredFunctionState::Failed;
                }
            }
        }
        UnpoweredFunctionState::Complete
    }

    fn reset(self: &mut Self, model: &Self::Model) {
        self.state = InterrogateState::Uninitialized;
        self.wrapped.reset(model);
    }
}
