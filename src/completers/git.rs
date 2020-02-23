//! Defines a completer for Git branches and commits.

use std::any;
use std::process::Command;

use itertools::Itertools;
use termion::color;

use crate::core;

#[derive(Debug, PartialEq)]
enum GitBranchCompletionType {
    Head,
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

    fn display_string(&self) -> String {
        let mut color_string = "".to_owned();
        if self.kind == GitBranchCompletionType::Tag {
            color_string = format!("{}", color::Fg(color::Yellow));
        } else if self.kind == GitBranchCompletionType::Head {
            color_string = format!("{}", color::Fg(color::Red));
        } else if self.kind == GitBranchCompletionType::RemoteBranch {
            color_string = format!("{}", color::Fg(color::LightBlack));
        }
        format!(
            "{}{}{}",
            color_string,
            self.branch_name,
            color::Fg(color::Reset)
        )
    }

    fn as_any(&self) -> &dyn any::Any {
        self
    }
}

pub struct GitBranchCompleter {}

impl GitBranchCompleter {
    pub fn new() -> Self {
        Self {}
    }
}

impl core::Completer for GitBranchCompleter {
    fn name(&self) -> String {
        "br".to_owned()
    }

    fn fetching_completions_finished(&self) -> bool {
        true
    }

    fn fetch_completions(&mut self) -> Vec<core::CompletionBox> {
        let mut fetched_completions: Vec<core::CompletionBox> = Vec::new();
        let result = Command::new("git")
            .args(&["for-each-ref", "--format=%(objecttype) %(refname:strip=2)"])
            .output()
            .expect("failed to run git-for-each-ref");

        if result.status.success() {
            fetched_completions.push(Box::new(GitBranchCompletion {
                kind: GitBranchCompletionType::Head,
                branch_name: "HEAD".to_owned(),
            }));
            for line in String::from_utf8_lossy(&result.stdout).lines() {
                let tuple = line.split_whitespace().next_tuple();
                if let Some((ref_type, ref_name)) = tuple {
                    let compl_type = if ref_type == "commit" {
                        if ref_name.contains('/') {
                            GitBranchCompletionType::RemoteBranch
                        } else {
                            GitBranchCompletionType::Branch
                        }
                    } else {
                        GitBranchCompletionType::Tag
                    };
                    fetched_completions.push(Box::new(GitBranchCompletion {
                        kind: compl_type,
                        branch_name: ref_name.to_owned(),
                    }));
                }
            }
        }
        fetched_completions
    }

    fn descend(&self, completion: &dyn core::Completion) -> Option<Box<dyn core::Completer>> {
        let completion_any = completion.as_any();
        let branch_completion = completion_any
            .downcast_ref::<GitBranchCompletion>()
            .unwrap();
        Some(Box::new(GitCommitCompleter::new(
            branch_completion.branch_name.as_str(),
        )))
    }
}

struct GitCommitCompletion {
    hash: String,
    date: String,
    author: String,
    subject: String,
}

impl core::Completion for GitCommitCompletion {
    fn result_string(&self) -> String {
        self.hash.clone()
    }

    fn display_string(&self) -> String {
        format!(
            "{:10} {:12} {:25} {}",
            &self.hash, &self.date, &self.author, &self.subject
        )
    }

    fn search_string(&self) -> String {
        self.subject.clone()
    }

    fn as_any(&self) -> &dyn any::Any {
        self
    }
}

struct GitCommitCompleter {
    branch_name: String,
}

impl GitCommitCompleter {
    fn new<B: Into<String>>(branch_name: B) -> GitCommitCompleter {
        GitCommitCompleter {
            branch_name: branch_name.into(),
        }
    }
}

impl core::Completer for GitCommitCompleter {
    fn name(&self) -> String {
        "co".to_owned()
    }

    fn fetching_completions_finished(&self) -> bool {
        true
    }

    fn fetch_completions(&mut self) -> Vec<core::CompletionBox> {
        let mut fetched_completions: Vec<core::CompletionBox> = Vec::new();
        let result = Command::new("git")
            .args(&[
                "log",
                "--format=%h%x09%ad%x09%an%x09%s",
                "--date=short",
                &self.branch_name,
            ])
            .output()
            .expect("failed to run git-log");

        if result.status.success() {
            for line in String::from_utf8_lossy(&result.stdout).lines() {
                let tuple = line.split('\t').next_tuple();
                if let Some((hash, date, author, subject)) = tuple {
                    fetched_completions.push(Box::new(GitCommitCompletion {
                        hash: hash.to_owned(),
                        date: date.to_owned(),
                        author: author.to_owned(),
                        subject: subject.to_owned(),
                    }));
                }
            }
        }
        fetched_completions
    }
}
