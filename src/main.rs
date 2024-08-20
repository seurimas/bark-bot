use std::env::args;

mod bt;
mod prelude;

fn main() {
    env_logger::init();

    let tree_path = args().nth(1).expect("Expected tree argument");
    let tree = std::fs::read_to_string(tree_path).expect("Failed to read tree file");
    let tree: bt::BarkDef = serde_json::from_str(&tree).expect("Failed to parse tree file");
    let mut tree = tree.create_tree();
    let mut controller = bt::BarkController::new();
    let model = bt::BarkModel::new();
    let mut state = tree.resume_with(&model, &mut controller);
    println!("{:?}", controller.text_variables);
    println!("{:?}", controller.prompts);
}
