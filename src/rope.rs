use std::rc::Rc;

enum Node {
    Leaf(String),
    Internal {
        left: Rc<Node>,
        right: Rc<Node>,
        weight: usize,
    },
}

impl Node {
    fn len(&self) -> usize {
        match self {
            Node::Leaf(s) => s.len(),
            Node::Internal { weight, right, .. } => weight + right.len(),
        }
    }

    fn new_leaf(text: &str) -> Rc<Self> {
        Rc::new(Node::Leaf(text.to_string()))
    }

    fn new_internal(left: Rc<Self>, right: Rc<Self>) -> Rc<Self> {
        Rc::new(Node::Internal {
            weight: left.len(),
            left,
            right,
        })
    }
}

pub struct Rope {
    root: Rc<Node>,
}

impl Rope {
    pub fn new(text: &str) -> Self {
        Self {
            root: Node::new_leaf(text),
        }
    }

    pub fn len(&self) -> usize {
        self.root.len()
    }

    pub fn insert(&mut self, index: usize, text: &str) {
        self.root = self.insert_rec(&self.root, index, text);
    }
}
