use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveFile<TC: ToolCaller> {
    pub path: TextValue,
    pub content: TextValue,
    #[serde(skip)]
    pub _phantom: std::marker::PhantomData<TC>,
}

impl<TC: ToolCaller> BehaviorTree for SaveFile<TC> {
    type Controller = BarkController;
    type Model = BarkModel<TC>;

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
                // eprintln!("Failed to save file: {:?}", err);
                BarkState::Failed
            }
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveIndexedFile<TC: ToolCaller> {
    pub path: TextValue,
    pub content: TextValue,
    pub index: usize,
    #[serde(skip)]
    pub _phantom: std::marker::PhantomData<TC>,
}

impl<TC: ToolCaller> BehaviorTree for SaveIndexedFile<TC> {
    type Controller = BarkController;
    type Model = BarkModel<TC>;

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

            Err(_err) => {
                // eprintln!("Failed to save file: {:?}", err);
                BarkState::Failed
            }
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // We don't reset the index. It will be used for the next save operation.
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadFile<TC: ToolCaller> {
    pub path: TextValue,
    pub content: VariableId,
    #[serde(skip)]
    pub _phantom: std::marker::PhantomData<TC>,
}

impl<TC: ToolCaller> BehaviorTree for LoadFile<TC> {
    type Controller = BarkController;
    type Model = BarkModel<TC>;

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
            Err(_err) => {
                // eprintln!("Failed to load file: {:?}", err);
                BarkState::Failed
            }
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadIndexedFile<TC: ToolCaller> {
    pub path: TextValue,
    pub content: VariableId,
    pub index: usize,
    #[serde(skip)]
    pub _phantom: std::marker::PhantomData<TC>,
}

impl<TC: ToolCaller> BehaviorTree for LoadIndexedFile<TC> {
    type Controller = BarkController;
    type Model = BarkModel<TC>;

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
            Err(_err) => {
                // eprintln!("Failed to load file: {:?}", err);
                BarkState::Failed
            }
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DumpState<TC: ToolCaller> {
    pub path: TextValue,
    #[serde(skip)]
    pub _phantom: std::marker::PhantomData<TC>,
}

impl<TC: ToolCaller> BehaviorTree for DumpState<TC> {
    type Controller = BarkController;
    type Model = BarkModel<TC>;

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
            Err(_err) => {
                // eprintln!("Failed to save state: {:?}", err);
                BarkState::Failed
            }
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
