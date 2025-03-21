use std::{env::args, process::ExitCode};

use bark_bot::{
    bt::BarkModelConfig,
    prelude::{read_tree, BarkState, TREE_ROOT},
};

#[tokio::main]
async fn main() -> ExitCode {
    env_logger::init();

    let tree_path = args().nth(1).expect("Expected tree argument");
    let model_config = args()
        .nth(2)
        .map(|s| {
            let config_str = std::fs::read_to_string(s).expect("Failed to read model config file");
            serde_json::from_str(&config_str).expect("Failed to parse model config")
        })
        .unwrap_or_else(|| BarkModelConfig::get_from_env());
    let tree_root = args().nth(3).unwrap_or_else(|| {
        std::path::Path::new(&tree_path)
            .parent()
            .unwrap()
            .to_string_lossy()
            .to_string()
    });
    let gas: i32 = args()
        .nth(4)
        .map(|s| s.parse().expect("Failed to parse gas"))
        .unwrap_or(10000);
    TREE_ROOT.set(tree_root).expect("Failed to set TREE_ROOT");
    let tree = read_tree(&tree_path);
    let mut tree = tree.create_tree();
    let mut controller = bark_bot::bt::BarkController::new();
    let mut gas = Some(gas);
    let model = bark_bot::bt::BarkModel::new(model_config);
    let mut state = tree.resume_with(&model, &mut controller, &mut gas, &mut None);
    while state == BarkState::Waiting {
        state = tree.resume_with(&model, &mut controller, &mut gas, &mut None);
    }
    println!("State: {:?}", controller);
    match state {
        BarkState::Complete => ExitCode::SUCCESS,
        BarkState::Failed => ExitCode::FAILURE,
        _ => panic!("Unexpected state: {:?}", state),
    }
}
