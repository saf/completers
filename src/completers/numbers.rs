//! This defines a completer for numbers which is used for testing the
//! completers API.

use std::any;

use super::super::core;

pub struct NumCompletion(String);

impl core::Completion for NumCompletion {
    fn result_string(&self) -> String {
        self.0.clone()
    }

    fn as_any(&self) -> &any::Any {
        self
    }
}

pub struct NumCompleter {
    count: usize,
}

impl NumCompleter {
    pub fn new(count: usize) -> NumCompleter {
        NumCompleter { count: count }
    }
}

impl core::Completer for NumCompleter {
    fn completions(&self) -> Vec<Box<core::Completion>> {
        let mut boxes: Vec<Box<core::Completion>> = vec![];
        for b in (0..self.count).map(|n| format!("{}", n)).map(|s| Box::new(NumCompletion(s))) {
            boxes.push(b);
        }
        boxes
    }
}
