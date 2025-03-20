use crate::prelude::*;

pub struct RepeatUntil {
    pub in_condition: bool,
    pub condition:
        Box<dyn BehaviorTree<Model = BarkModel, Controller = BarkController> + Send + Sync>,
    pub action: Box<dyn BehaviorTree<Model = BarkModel, Controller = BarkController> + Send + Sync>,
}

impl RepeatUntil {
    pub fn new(
        condition: Box<
            dyn BehaviorTree<Model = BarkModel, Controller = BarkController> + Send + Sync,
        >,
        action: Box<dyn BehaviorTree<Model = BarkModel, Controller = BarkController> + Send + Sync>,
    ) -> Self {
        Self {
            in_condition: false,
            condition,
            action,
        }
    }
}

impl BehaviorTree for RepeatUntil {
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
            if self.in_condition {
                let condition_state = self.condition.resume_with(model, controller, gas, audit);
                match condition_state {
                    BehaviorTreeState::Complete => {
                        return BehaviorTreeState::Complete;
                    }
                    BehaviorTreeState::Failed => {
                        self.in_condition = false;
                    }
                    _ => {
                        return condition_state;
                    }
                }
            }
            let action_state = self.action.resume_with(model, controller, gas, audit);
            match action_state {
                BehaviorTreeState::Complete => {
                    self.in_condition = true;
                }
                _ => {
                    return action_state;
                }
            }
        }
    }

    fn reset(self: &mut Self, model: &Self::Model) {
        self.in_condition = true;
        self.condition.reset(model);
        self.action.reset(model);
    }
}
