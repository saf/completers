//! Defines a completer for Git branches and commits.

use std::any;
use std::process::Command;
use std::sync::Arc;

use itertools::Itertools;

use core;

use std::io;
use std::io::Write;

enum GitBranchCompletionType {
    Branch,
    RemoteBranch,
    Tag,
}

struct GitBranchCompletion {
    kind: GitBranchCompletionType,
    branch_name: String,
}

impl core::Completion for GitBranchCompletion {
    fn result_string(&self) -> String {
        self.branch_name.clone()
    }

    fn as_any(&self) -> &any::Any {
        self
    }
}

pub struct GitBranchCompleter {
    all_completions: Vec<core::CompletionBox>,
    query: String,
    filtered_completions: Vec<core::CompletionBox>,
}

impl GitBranchCompleter {
    pub fn new() -> GitBranchCompleter {
        GitBranchCompleter {
            all_completions: vec![],
            query: String::new(),
            filtered_completions: vec![],
        }
    }
    
    fn filter_completions(&self, completions: &[core::CompletionBox]) -> Vec<core::CompletionBox> {
        let mut result = Vec::new();
        for completion_arc in completions {
            if completion_arc.result_string().contains(&self.query) {
                result.push(completion_arc.clone());
            }
        }
        result
    }
}

impl core::Completer for GitBranchCompleter {
    fn completions(&self) -> &[core::CompletionBox] {
        self.filtered_completions.as_slice()
    }

    fn fetching_completions_finished(&self) -> bool {
        true
    }

    fn fetch_completions(&mut self) {
        let result = Command::new("git").args(
            &["for-each-ref", "--format=%(objecttype) %(refname:strip=2)"]
        ).output().expect("failed to run git-for-each-ref");
        
        if result.status.success() {
            self.all_completions.push(Arc::new(GitBranchCompletion {
                kind: GitBranchCompletionType::Branch,
                branch_name: "HEAD".to_owned(),
            }));
            for line in String::from_utf8_lossy(&result.stdout).lines() {
                let tuple = line.split_whitespace().next_tuple();
                if let Some((ref_type, ref_name)) = tuple {
                    let compl_type =
                        if ref_type == "commit" {
                            if ref_name.contains('/') {
                                GitBranchCompletionType::RemoteBranch
                            } else {
                                GitBranchCompletionType::Branch
                            }
                        } else {
                            GitBranchCompletionType::Tag
                        };
                    self.all_completions.push(Arc::new(GitBranchCompletion {
                        kind: compl_type,
                        branch_name: ref_name.to_owned(),
                    }));
                }
            }
        }

        self.filtered_completions = self.filter_completions(self.all_completions.as_slice());
    }

    fn set_query(&mut self, query: String) {
        self.query = query;
        self.filtered_completions = self.filter_completions(self.all_completions.as_slice());
    }
}
