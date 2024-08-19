use crate::prelude::*;

pub struct Interrogate {
    subnodes: Vec<BarkFunction>,
    position: usize,
}

impl Interrogate {
    pub fn new(subnodes: Vec<BarkFunction>) -> Self {
        Self {
            subnodes,
            position: 0,
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
        while self.position < self.subnodes.len() {
            let node = &mut self.subnodes[self.position];
            let result = node.resume_with(model, controller);
            match result {
                UnpoweredFunctionState::Complete => {
                    self.position += 1;
                }
                _ => {
                    return result;
                }
            }
        }
        UnpoweredFunctionState::Complete
    }

    fn reset(self: &mut Self, model: &Self::Model) {
        for node in self.subnodes.iter_mut() {
            node.reset(model);
        }
    }
}
