use crate::prelude::*;

pub struct Interrogate {
    state: InterrogateState,
    current: String,
    remaining: String,
    text_value: TextValue,
    wrapped: BarkFunction,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum InterrogateState {
    Uninitialized,
    Waited,
    NotWaited,
}

impl Interrogate {
    pub fn new(text_value: TextValue, mut wrapped: Vec<BarkFunction>) -> Self {
        Self {
            state: InterrogateState::Uninitialized,
            current: "".to_string(),
            remaining: "".to_string(),
            text_value,
            wrapped: wrapped.pop().unwrap(),
        }
    }
}

impl BehaviorTree for Interrogate {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
        _gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        if self.state == InterrogateState::Uninitialized {
            let output = controller.get_text(&self.text_value);
            self.current = String::new();
            self.remaining = output;
            self.state = InterrogateState::NotWaited;
        }
        while self.remaining.len() > 0 {
            match self.state {
                InterrogateState::NotWaited => {
                    // The node has not waited, so it has completed for the previous output.
                    self.wrapped.reset(model);
                    let newline = self.remaining.find('\n');
                    match newline {
                        Some(index) => {
                            self.current = self.remaining[..index].to_string();
                            self.remaining = self.remaining[index + 1..].to_string();
                        }
                        None => {
                            self.current = self.remaining.clone();
                            self.remaining = "".to_string();
                        }
                    }
                    controller
                        .text_variables
                        .insert(VariableId::LoopValue, self.current.clone());
                }
                _ => {}
            }
            let result = self.wrapped.resume_with(model, controller, _gas, _audit);
            match result {
                BarkState::Complete => {
                    self.state = InterrogateState::NotWaited;
                }
                BarkState::Waiting => {
                    self.state = InterrogateState::Waited;
                    return BarkState::Waiting;
                }
                BarkState::Failed => {
                    // XXX: Do we need to reset here?
                    return BarkState::Failed;
                }
                BarkState::WaitingForGas => {
                    return BarkState::WaitingForGas;
                }
            }
        }
        BarkState::Complete
    }

    fn reset(self: &mut Self, model: &Self::Model) {
        self.state = InterrogateState::Uninitialized;
        self.wrapped.reset(model);
    }
}
