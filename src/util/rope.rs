use std::fmt;
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

    fn len(&self) -> usize {
        match self {
            Node::Leaf(s) => s.len(),
            Node::Internal { weight, right, .. } => weight + right.len(),
        }
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

    fn insert_rec(&self, node: &Rc<Node>, index: usize, text: &str) -> Rc<Node> {
        match node.as_ref() {
            Node::Leaf(existing) => {
                let mut new_text = String::with_capacity(existing.len() + text.len());
                new_text.push_str(&existing[..index]);
                new_text.push_str(text);
                new_text.push_str(&existing[index..]);
                Node::new_leaf(&new_text)
            }
            Node::Internal { left, right, weight } => {
                if index < *weight {
                    let left = self.insert_rec(left, index, text);
                    return Node::new_internal(left, right.clone());
                } else {
                    let right = self.insert_rec(right, index - weight, text);
                    return Node::new_internal(left.clone(), right);
                }
            }
        }
    }

    fn collect_text(&self, node: &Rc<Node>, result: &mut String) {
        match node.as_ref() {
            Node::Leaf(text) => result.push_str(text),
            Node::Internal { left, right, .. } => {
                self.collect_text(left, result);
                self.collect_text(right, result);
            }
        }
    }
}

impl fmt::Display for Rope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut result = String::new();
        self.collect_text(&self.root, &mut result);
        f.write_str(&result)
    }
}
