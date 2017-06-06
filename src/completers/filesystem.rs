//! This defines the completer which provides completions of file
//! names existing in the local file system.

use std::any;
use std::fs;
use std::path;

use termion::color;

use super::super::core;

#[derive(PartialEq)]
enum FsEntryType {
    Directory,
    File,
    Error,
}

struct FsCompletion {
    relative_path: path::PathBuf,
    entry_type: FsEntryType,
}

impl FsCompletion {
    fn get_completions(dir_path: &path::Path) -> Vec<Box<core::Completion>> {
        let mut boxes: Vec<Box<core::Completion>> = Vec::new();
        let mut entries = fs::read_dir(dir_path).unwrap();
        while let Some(Ok(entry)) = entries.next() {
            let entry_type = match entry.metadata() {
                Ok(md) =>
                    if md.is_dir() {
                        FsEntryType::Directory
                    } else {
                        FsEntryType::File
                    },
                _ => FsEntryType::Error
            };

            let here_prefix = path::Path::new("./");
            let mut path = dir_path.join(entry.file_name());
            if path.starts_with(here_prefix) {
                path = path.strip_prefix(here_prefix).unwrap().to_path_buf();
            }
            if let Some(s) = path.file_name().and_then(|f| f.to_str()) {
                if s.starts_with(".") {
                    continue;
                }
            }
            
            boxes.push(Box::new(FsCompletion {
                relative_path: path,
                entry_type: entry_type,
            }));
        }
        boxes.sort_by_key(|b| b.result_string());
        boxes
    }
}

impl core::Completion for FsCompletion {
    fn result_string(&self) -> String {
        self.relative_path.to_str().unwrap().to_string()
    }

    fn display_string(&self) -> String {
        if self.entry_type == FsEntryType::Directory {
            format!("{}{}{}", color::Fg(color::Blue),
                    self.result_string(), color::Fg(color::Reset))
        } else {
            self.result_string()
        }
    }

    fn as_any(&self) -> &any::Any {
        self
    }
}

pub struct FsCompleter {
    current_path: path::PathBuf,
}

impl FsCompleter {
    pub fn new() -> FsCompleter {
        FsCompleter { current_path: path::PathBuf::from(".") }
    }
}

impl core::Completer for FsCompleter {
    fn completions(&self) -> Vec<Box<core::Completion>> {
        FsCompletion::get_completions(self.current_path.as_path())
    }

    fn can_descend(&self, completion: &core::Completion) -> bool {
        let completion_any = completion.as_any();
        match completion_any.downcast_ref::<FsCompletion>() {
            Some(&FsCompletion { entry_type: FsEntryType::Directory, .. }) => true,
            _ => false
        }
    }

    fn descend(&mut self, completion: &core::Completion) {
        let completion_any = completion.as_any();
        let fs_completion = completion_any.downcast_ref::<FsCompletion>().unwrap();
        self.current_path.push(fs_completion.relative_path.file_name().unwrap());
    }

    fn can_ascend(&self) -> bool {
        self.current_path != path::Path::new("/")
    }

    fn ascend(&mut self) {
        if self.current_path.ends_with(path::Path::new(".")) {
            self.current_path = path::PathBuf::from("..");
        } else if self.current_path.ends_with(path::Path::new("..")) {
            self.current_path.push(path::Path::new(".."));
        } else {
            self.current_path.pop();
        }
        if self.current_path.canonicalize().unwrap() == path::Path::new("/") {
            self.current_path = path::PathBuf::from("/");
        }
    }
}
