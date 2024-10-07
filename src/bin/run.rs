use std::{env::args, process::ExitCode};

use bark_bot::prelude::{read_tree, BarkState};

fn main() -> ExitCode {
    env_logger::init();

    let tree_path = args().nth(1).expect("Expected tree argument");
    let tree = read_tree(&tree_path);
    let mut tree = tree.create_tree();
    let mut controller = bark_bot::bt::BarkController::new();
    let mut gas = Some(20);
    let model = bark_bot::bt::BarkModel::new();
    let mut state = tree.resume_with(&model, &mut controller, &mut gas, &mut None);
    while state == BarkState::Waiting {
        state = tree.resume_with(&model, &mut controller, &mut gas, &mut None);
    }
    match state {
        BarkState::Complete => ExitCode::SUCCESS,
        BarkState::Failed => ExitCode::FAILURE,
        _ => panic!("Unexpected state: {:?}", state),
    }
}
