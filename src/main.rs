use std::env::args;

use rusqlite::{ffi::sqlite3_auto_extension, Connection};
use sqlite_vec::sqlite3_vec_init;
use zerocopy::AsBytes;

mod bt;
mod prelude;

fn main() {
    env_logger::init();
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
    }
    // future database connection will now automatically include sqlite-vec functions!
    let db = Connection::open_in_memory().unwrap();
    let vec_version: String = db
        .query_row("select vec_version()", [], |x| x.get(0))
        .unwrap();

    println!("vec_version={vec_version}");

    let tree_path = args().nth(1).expect("Expected tree argument");
    let tree = std::fs::read_to_string(tree_path).expect("Failed to read tree file");
    let tree: bt::BarkDef = serde_json::from_str(&tree).expect("Failed to parse tree file");
    let mut tree = tree.create_tree();
    let mut controller = bt::BarkController::new();
    let model = bt::BarkModel::new();
    let mut state = tree.resume_with(&model, &mut controller);
    println!("{:?}", controller.text_variables);
    println!("{:?}", controller.prompts);
    println!("{:?}", controller.embedding_variables);
    println!("{:?}", state);
}
