use std::env::args;

use prelude::read_tree;

mod bt;
mod prelude;

fn main() {
    env_logger::init();

    let tree_path = args().nth(1).expect("Expected tree argument");
    let tree = read_tree(&tree_path);
    let mut tree = tree.create_tree();
    let mut controller = bt::BarkController::new();
    let model = bt::BarkModel::new();
    let mut state = tree.resume_with(&model, &mut controller);
    println!("{:?}", controller.text_variables);
    println!("{:?}", controller.prompts);
    println!("{:?}", controller.embedding_variables);
    println!("{:?}", state);
}
