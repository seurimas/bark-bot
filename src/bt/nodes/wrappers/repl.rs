use crate::prelude::*;

pub struct Repl {
    prompt: TextValue,
    text_values: Vec<TextValue>,
    best_index: Option<usize>,
    nodes: Vec<Box<dyn BehaviorTree<Model = BarkModel, Controller = BarkController> + Send + Sync>>,
}

impl Repl {
    pub fn new(
        prompt: TextValue,
        text_values: Vec<TextValue>,
        nodes: Vec<
            Box<dyn BehaviorTree<Model = BarkModel, Controller = BarkController> + Send + Sync>,
        >,
    ) -> Self {
        if nodes.len() != text_values.len() {
            panic!("REPL nodes and text_values must have the same length");
        }
        Self {
            prompt,
            text_values,
            best_index: None,
            nodes,
        }
    }
}

impl BehaviorTree for Repl {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        model: &Self::Model,
        controller: &mut Self::Controller,
        gas: &mut Option<i32>,
        audit: &mut Option<BehaviorTreeAudit>,
    ) -> BehaviorTreeState {
        loop {
            if self.best_index.is_none() {
                println!("{}", controller.get_text(&self.prompt));
                let input = model.read_stdin(true);
                let idx = self
                    .text_values
                    .iter()
                    .position(|v| controller.get_text(v).eq_ignore_ascii_case(&input));
                if let Some(idx) = idx {
                    self.best_index = Some(idx);
                } else {
                    return BehaviorTreeState::Failed;
                }
            }
            let node = self.nodes.get_mut(self.best_index.unwrap()).unwrap();
            match node.resume_with(model, controller, gas, audit) {
                BehaviorTreeState::Complete => {
                    self.best_index = None;
                    // Continue on!
                    continue;
                }
                state => {
                    return state;
                }
            }
        }
    }

    fn reset(self: &mut Self, model: &Self::Model) {
        self.best_index = None;
        for node in self.nodes.iter_mut() {
            node.reset(model);
        }
    }
}
