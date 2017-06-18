//! This defines a completer for numbers which is used for testing the
//! completers API.

use std::any;

use core;

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
    completions: Vec<core::CompletionBox>,
}

impl NumCompleter {
    pub fn new(count: usize) -> NumCompleter {
        let mut completions: Vec<core::CompletionBox> = vec![];
        for b in (0..count).map(|n| format!("{}", n)).map(|s| Box::new(NumCompletion(s))) {
            completions.push(b);
        }
        NumCompleter { completions: completions }
    }
}

impl core::Completer for NumCompleter {
    fn completions(&self) -> &[core::CompletionBox] {
        &self.completions
    }
}
