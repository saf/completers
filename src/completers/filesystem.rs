//! This defines the completer which provides completions of file
//! names existing in the local file system.

use std::any;
use std::fs;
use std::path;
use std::rc::Rc;
use std::collections::vec_deque::VecDeque;

use termion::color;

use core;

const DIRECTORY_DEPTH_LIMIT: usize = 4;

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

impl core::Completion for FsCompletion {
    fn result_string(&self) -> String {
        self.relative_path.to_string_lossy().into_owned()
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

/// Type representing an entry in the BFS queue of directory enumeration.
///
/// The first element is a directory path, and the second element signifies
/// the depth of the directory in the search.
struct DirectoryQueueEntry(path::PathBuf, usize);

pub struct FsCompleter {
    current_path: path::PathBuf,
    completions: core::Completions,
}

impl FsCompleter {
    pub fn new() -> FsCompleter {
        let mut completer = FsCompleter {
            current_path: path::PathBuf::from("."),
            completions: core::Completions::new(),
        };
        completer.update_completions();
        completer
    }

    fn directory_bfs(queue: &mut VecDeque<DirectoryQueueEntry>) -> core::Completions {
        let queue_entry = queue.pop_front();
        if let None = queue_entry {
            return core::Completions::new();
        }
        let DirectoryQueueEntry(dir_path, depth) = queue_entry.unwrap();
        let mut completions = core::Completions::new();
        let mut read_dir_result = fs::read_dir(&dir_path);
        if let Err(_) = read_dir_result {
            return core::Completions::new();
        }
        let mut entries = read_dir_result.unwrap();
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

            if entry_type == FsEntryType::Directory && depth < DIRECTORY_DEPTH_LIMIT {
                queue.push_back(DirectoryQueueEntry(path.clone(), depth + 1));
            }

            completions.push(Rc::new(FsCompletion {
                relative_path: path,
                entry_type: entry_type,
            }));
        }
        completions.sort_by_key(|b| b.result_string());
        completions
    }

    fn update_completions(&mut self) {
        let mut dir_queue: VecDeque<DirectoryQueueEntry> = VecDeque::new();
        dir_queue.push_back(DirectoryQueueEntry(self.current_path.clone(), 0));
        let mut completions = core::Completions::new();
        while !dir_queue.is_empty() {
            completions.extend(FsCompleter::directory_bfs(&mut dir_queue));
        }
        self.completions = completions;
    }
}

impl core::Completer for FsCompleter {
    fn get_completions(&self) -> core::GetCompletionsResult {
        core::GetCompletionsResult(self.completions.clone(), true)
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
        self.update_completions();
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
        self.update_completions();
    }
}
