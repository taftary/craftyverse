mod node;

use node::Node;

fn main() {
    let node = Node::new("root");
    println!("Created node: {}", node.name);
}
