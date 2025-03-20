use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SaveFile {
    pub path: TextValue,
    pub content: TextValue,
}

impl BehaviorTree for SaveFile {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
        _gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        let path = controller.get_text(&self.path);
        let content = controller.get_text(&self.content);
        match std::fs::write(&path, &content) {
            Ok(_) => BarkState::Complete,
            Err(err) => {
                eprintln!("Failed to save file: {:?}", err);
                BarkState::Failed
            }
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoadFile {
    pub path: TextValue,
    pub content: VariableId,
}

impl BehaviorTree for LoadFile {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
        _gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        let path = controller.get_text(&self.path);
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                controller
                    .text_variables
                    .insert(self.content.clone(), content);
                BarkState::Complete
            }
            Err(err) => {
                eprintln!("Failed to load file: {:?}", err);
                BarkState::Failed
            }
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[cfg(feature = "arc")]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReadArc {
    pub path: TextValue,
    pub content: VariableId,
}

#[cfg(feature = "arc")]
impl BehaviorTree for ReadArc {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
        _gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        let path = controller.get_text(&self.path);
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let data: TrainingData = serde_json::from_str(&content).unwrap();
                controller.arc_variables.insert(self.content.clone(), data);
                BarkState::Complete
            }
            Err(err) => {
                eprintln!("Failed to load file: {:?}", err);
                BarkState::Failed
            }
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
