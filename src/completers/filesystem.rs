//! This defines the completer which provides completions of file
//! names existing in the local file system.

use std::fs;
use std::path;

use termion::color;

use super::super::core;

#[derive(PartialEq)]
enum FsCompletionType {
    Directory,
    File,
    Error,
}

struct FsCompletion {
    relative_path: path::PathBuf,
    entry_type: FsCompletionType,
}

impl FsCompletion {
    fn get_completions(dir_path: &path::Path) -> Vec<Box<core::Completion>> {
        let mut boxes: Vec<Box<core::Completion>> = Vec::new();
        let mut entries = fs::read_dir(dir_path).unwrap();
        while let Some(Ok(entry)) = entries.next() {
            let entry_type = match entry.metadata() {
                Ok(md) =>
                    if md.is_dir() {
                        FsCompletionType::Directory
                    } else {
                        FsCompletionType::File
                    },
                _ => FsCompletionType::Error
            };

            let here_prefix = path::Path::new("./");
            let mut path = dir_path.join(entry.file_name());
            if path.starts_with(here_prefix) {
                path = path.strip_prefix(here_prefix).unwrap().to_path_buf();
            }
            
            boxes.push(Box::new(FsCompletion {
                relative_path: path,
                entry_type: entry_type,
            }));
        }
        boxes
    }
}

impl core::Completion for FsCompletion {
    fn result_string(&self) -> String {
        self.relative_path.to_str().unwrap().to_string()
    }

    fn display_string(&self) -> String {
        if self.entry_type == FsCompletionType::Directory {
            format!("{}{}{}", color::Fg(color::Blue),
                    self.result_string(), color::Fg(color::Reset))
        } else {
            self.result_string()
        }
    }

    fn has_children(&self) -> bool {
        self.entry_type == FsCompletionType::Error
    }

    fn children(&self) -> Vec<Box<core::Completion>> {
        FsCompletion::get_completions(self.relative_path.as_path())
    }
}

pub struct FsCompleter {
    completions: Vec<Box<core::Completion>>,
}

impl FsCompleter {
    pub fn new() -> FsCompleter {
        FsCompleter {
            completions: FsCompletion::get_completions(path::Path::new(".")),
        }
    }
}

impl core::Completer for FsCompleter {
    fn completions(&self) -> &[Box<core::Completion>] {
        self.completions.as_slice()
    }
}
