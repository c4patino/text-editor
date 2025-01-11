use color_eyre::Report;
use crossterm::event::KeyEvent;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::editor::{Editor, Mode};

type ActionFn = dyn FnMut(&mut Editor) -> Result<(), Report>;

#[derive(Clone)]
struct KeyNode {
    children: HashMap<KeyEvent, Rc<RefCell<KeyNode>>>,
    action: Option<Rc<RefCell<ActionFn>>>,
}

pub struct Keymap {
    root: HashMap<Mode, Rc<RefCell<KeyNode>>>,
    current: Option<Rc<RefCell<KeyNode>>>,
}

impl KeyNode {
    pub fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self { children: HashMap::new(), action: None }))
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    pub fn insert(&mut self, sequence: Vec<KeyEvent>, action: Rc<RefCell<ActionFn>>) {
        if sequence.is_empty() {
            self.action = Some(action);
            return;
        }

        let (key, sequence) = sequence.split_first().unwrap();
        let next_node = self.children.entry(key.clone()).or_insert_with(KeyNode::new);

        next_node.borrow_mut().insert(sequence.to_vec(), action)
    }
}

impl Keymap {
    pub fn new() -> Self {
        Self { root: HashMap::new(), current: None }
    }

    pub fn add_keybind<F>(&mut self, modes: Vec<Mode>, sequence: Vec<KeyEvent>, action: F)
    where
        F: FnMut(&mut Editor) -> Result<(), Report> + 'static,
    {
        let action = Rc::new(RefCell::new(action));

        for mode in modes {
            let mut root_node = self.root.entry(mode).or_insert_with(KeyNode::new).borrow_mut();
            root_node.insert(sequence.clone(), action.clone());
        }
    }

    pub fn traverse(&mut self, mode: &Mode, event: KeyEvent) -> Result<Option<KeyEvent>, Report> {
        let current_node = match self.current {
            Some(ref node) => node.clone(),
            None => self.root.get(mode).unwrap().clone(),
        };

        let next_node = match current_node.borrow().children.get(&event) {
            Some(node) => node.clone(),
            None => return Ok(Some(event)),
        };

        self.current = Some(next_node);
        Ok(None)
    }

    pub fn is_leaf(&self) -> bool {
        match self.current {
            Some(ref node) => node.borrow().is_leaf(),
            None => false,
        }
    }

    pub fn get_action(&self) -> Option<Rc<RefCell<ActionFn>>> {
        if self.current.is_none() {
            return None;
        }

        self.current.as_ref()?.borrow().action.clone()
    }

    pub fn clear(&mut self) {
        self.current = None;
    }

    pub fn is_empty(&self) -> bool {
        self.current.is_none()
    }
}
