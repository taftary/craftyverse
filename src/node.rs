pub struct Node {
    pub name: String,
}

impl Node {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}
