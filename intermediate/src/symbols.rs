use crate::Value;

use std::collections::HashMap;

pub struct Symbols<'a> {
    names: HashMap<String, Value>,
    parent: Option<&'a Symbols<'a>>,
}

impl<'a> Symbols<'a> {
    pub fn new(parent: Option<&'a Symbols<'a>>) -> Self {
        Self {
            names: HashMap::new(),
            parent,
        }
    }

    pub fn define(&mut self, name: String, val: Value) {
        self.names.insert(name, val);
    }

    pub fn resolve(&self, name: &str) -> Option<Value> {
        self.names
            .get(name)
            .cloned()
            .or_else(|| self.parent.and_then(|p| p.resolve(name)))
    }
}
