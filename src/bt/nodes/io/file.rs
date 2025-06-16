use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveIndexedFile {
    pub path: TextValue,
    pub content: TextValue,
    pub index: usize,
}

impl BehaviorTree for SaveIndexedFile {
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
        // Append the index to the file name
        let path = format!("{}-{}", path, self.index);
        // Get the content from the controller
        let content = controller.get_text(&self.content);
        // Write the content to the file
        match std::fs::write(&path, &content) {
            Ok(_) => {
                self.index += 1; // Increment the index for the next save
                BarkState::Complete
            }

            Err(err) => {
                eprintln!("Failed to save file: {:?}", err);
                BarkState::Failed
            }
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // We don't reset the index. It will be used for the next save operation.
    }
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadIndexedFile {
    pub path: TextValue,
    pub content: VariableId,
    pub index: usize,
}

impl BehaviorTree for LoadIndexedFile {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
        _gas: &mut Option<i32>,
        mut _audit: &mut Option<BehaviorTreeAudit>,
    ) -> BarkState {
        // Append the index to the file name
        let path = format!("{}-{}", controller.get_text(&self.path), self.index);
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                // Insert the content into the controller's text variables
                controller
                    .text_variables
                    .insert(self.content.clone(), content);
                self.index += 1; // Increment the index for the next load
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

#[derive(Debug, Serialize, Deserialize)]
pub struct DumpState {
    pub path: TextValue,
}

impl BehaviorTree for DumpState {
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
        match std::fs::write(&path, serde_json::to_string(controller).unwrap()) {
            Ok(_) => BarkState::Complete,
            Err(err) => {
                eprintln!("Failed to save state: {:?}", err);
                BarkState::Failed
            }
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
