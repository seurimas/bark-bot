use std::{env::args, process::ExitCode};

use bark_bot::prelude::{read_tree, UnpoweredFunctionState};

fn main() -> ExitCode {
    env_logger::init();

    let tree_path = args().nth(1).expect("Expected tree argument");
    let tree = read_tree(&tree_path);
    let mut tree = tree.create_tree();
    let mut controller = bark_bot::bt::BarkController::new();
    let model = bark_bot::bt::BarkModel::new();
    let mut state = tree.resume_with(&model, &mut controller);
    while state == UnpoweredFunctionState::Waiting {
        state = tree.resume_with(&model, &mut controller);
    }
    match state {
        UnpoweredFunctionState::Complete => ExitCode::SUCCESS,
        UnpoweredFunctionState::Failed => ExitCode::FAILURE,
        _ => panic!("Unexpected state: {:?}", state),
    }
}
