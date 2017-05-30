//! This defines a completer for numbers which is used for testing the
//! completers API.

use super::super::core;

pub struct NumCompletion(String);

impl core::Completion for NumCompletion {
    fn result_string(&self) -> &str {
        self.0.as_str()
    }

    fn has_children(&self) -> bool {
        false
    }

    fn children(&self) -> Vec<Box<core::Completion>> {
        vec![]
    }
}

pub struct NumCompleter {
    completions: Vec<Box<core::Completion>>,
}

impl NumCompleter {
    pub fn new(count: usize) -> NumCompleter {
        let mut boxes: Vec<Box<core::Completion>> = vec![];
        for b in (0..count).map(|n| format!("{}", n)).map(|s| Box::new(NumCompletion(s))) {
            boxes.push(b);
        }
        NumCompleter { completions: boxes }
    }
}

impl core::Completer for NumCompleter {
    fn completions(&self) -> &[Box<core::Completion>] {
        self.completions.as_slice()
    }
}
