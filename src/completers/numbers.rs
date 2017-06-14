//! This defines a completer for numbers which is used for testing the
//! completers API.

use std::any;
use std::sync::Arc;

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
    count: usize,
}

impl NumCompleter {
    pub fn new(count: usize) -> NumCompleter {
        NumCompleter { count: count }
    }
}

impl core::Completer for NumCompleter {
    fn get_completions(&mut self) -> core::GetCompletionsResult {
        let mut completions: core::Completions = vec![];
        for rc in (0..self.count).map(|n| format!("{}", n)).map(|s| Arc::new(NumCompletion(s))) {
            completions.push(rc);
        }
        core::GetCompletionsResult(completions, true)
    }
}
