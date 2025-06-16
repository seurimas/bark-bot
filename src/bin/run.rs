use std::{env::args, process::ExitCode};

use bark_bot::{
    bt::BarkModelConfig,
    prelude::{read_tree, BarkState},
};

#[tokio::main]
async fn main() -> ExitCode {
    env_logger::init();

    let mut tree_path = args().nth(1).expect("Expected tree argument");
    let model_config = args()
        .nth(2)
        .map(|s| {
            let config_str = std::fs::read_to_string(s).expect("Failed to read model config file");
            serde_json::from_str(&config_str).expect("Failed to parse model config")
        })
        .unwrap_or_else(|| BarkModelConfig::get_from_env());
    let tree_root = args().nth(3).unwrap_or_else(|| {
        let new_tree_path = std::path::Path::new(&tree_path)
            .file_name()
            .expect("Failed to get file name from tree path")
            .to_string_lossy()
            .to_string();
        let tree_root = std::path::Path::new(&tree_path)
            .parent()
            .unwrap()
            .to_string_lossy()
            .to_string();
        tree_path = new_tree_path;
        tree_root
    });
    let gas: i32 = args()
        .nth(4)
        .map(|s| s.parse().expect("Failed to parse gas"))
        .unwrap_or(100000);
    let tree = read_tree(&tree_root, &tree_path);
    let mut tree = tree.create_tree();
    let mut controller = bark_bot::bt::BarkController::new();
    let mut gas = Some(gas);
    // let mut audit = Some(Default::default());
    let mut audit = None; // Disable audit for now, can be enabled later if needed
    let model = bark_bot::bt::BarkModel::new(model_config, tree_root).await;
    let mut state = tree.resume_with(&model, &mut controller, &mut gas, &mut audit);
    while state == BarkState::Waiting {
        state = tree.resume_with(&model, &mut controller, &mut gas, &mut audit);
    }
    println!("State: {:?} {:?}", state, controller);
    if let Some(audit) = audit {
        println!("Audit: {:?}", audit);
    }
    match state {
        BarkState::Complete => ExitCode::SUCCESS,
        BarkState::Failed => ExitCode::FAILURE,
        _ => panic!("Unexpected state: {:?}", state),
    }
}
