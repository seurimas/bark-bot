use std::{env::args, process::ExitCode};

use bark_bot::{
    bt::BarkModelConfig,
    prelude::{read_tree, BarkState},
};

fn main() -> ExitCode {
    env_logger::init();

    let tree_path = args().nth(1).expect("Expected tree argument");
    let model_config = args()
        .nth(2)
        .map(|s| {
            let config_str = std::fs::read_to_string(s).expect("Failed to read model config file");
            serde_json::from_str(&config_str).expect("Failed to parse model config")
        })
        .unwrap_or_else(|| BarkModelConfig::get_from_env());
    let tree = read_tree(&tree_path);
    let mut tree = tree.create_tree();
    let mut controller = bark_bot::bt::BarkController::new();
    let mut gas = Some(1000000);
    let model = bark_bot::bt::BarkModel::new(model_config);
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
