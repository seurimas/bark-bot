use crate::prelude::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SaveFile {
    pub path: TextValue,
    pub content: TextValue,
}

impl UnpoweredFunction for SaveFile {
    type Controller = BarkController;
    type Model = BarkModel;

    fn resume_with(
        self: &mut Self,
        _model: &Self::Model,
        controller: &mut Self::Controller,
    ) -> UnpoweredFunctionState {
        let path = controller.get_text(&self.path);
        let content = controller.get_text(&self.content);
        match std::fs::write(&path, &content) {
            Ok(_) => UnpoweredFunctionState::Complete,
            Err(err) => {
                eprintln!("Failed to save file: {:?}", err);
                UnpoweredFunctionState::Failed
            }
        }
    }

    fn reset(self: &mut Self, _model: &Self::Model) {
        // Nothing to do
    }
}
