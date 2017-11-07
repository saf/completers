//! This defines the completer which provides completions of file
//! names existing in the local file system.

use std::any;
use std::collections::vec_deque::VecDeque;
use std::fs;
use std::path;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

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

/// A structure representing the background fetching thread.
struct BgThread {
    pub thread: thread::JoinHandle<()>,
    pub request_send: mpsc::Sender<()>,
    pub response_recv: mpsc::Receiver<Option<Vec<core::CompletionBox>>>,
}

fn directory_bfs(queue: &mut VecDeque<DirectoryQueueEntry>) -> Vec<core::CompletionBox> {
    let queue_entry = queue.pop_front();
    if let None = queue_entry {
        return vec![];
    }
    let DirectoryQueueEntry(dir_path, depth) = queue_entry.unwrap();
    let mut completions: Vec<core::CompletionBox> = vec![];
    let read_dir_result = fs::read_dir(&dir_path);
    if let Err(_) = read_dir_result {
        return vec![];
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

        completions.push(Arc::new(FsCompletion {
            relative_path: path,
            entry_type: entry_type,
        }));
    }
    completions.sort_by_key(|c| c.result_string());
    completions
}

fn fetching_thread_routine(dir_path: path::PathBuf, request_recv: mpsc::Receiver<()>,
                           response_send: mpsc::Sender<Option<Vec<core::CompletionBox>>>) {
    let mut dir_queue: VecDeque<DirectoryQueueEntry> = VecDeque::new();
    dir_queue.push_back(DirectoryQueueEntry(dir_path, 0));
    let mut completions = Vec::new();
    while !dir_queue.is_empty() {
        completions.extend(directory_bfs(&mut dir_queue));
        match request_recv.try_recv() {
            Result::Ok(_) => {
                response_send.send(Some(completions)).unwrap();
                completions = Vec::new();
            },
            Result::Err(mpsc::TryRecvError::Empty) => {},
            Result::Err(mpsc::TryRecvError::Disconnected) => {
                return;
            }
        }
    }
    match request_recv.recv() {
        Result::Ok(_) => {
            response_send.send(Some(completions)).unwrap();
        }
        _ => {
            return;
        }
    }
    match request_recv.recv() {
        Result::Ok(_) => {
            response_send.send(None).unwrap();
        },
        Result::Err(_) => {
            return;
        }
    }
}

/// A structure representing the state of fetching completions for a
/// single level (directory).
///
/// The user may descend into a directory when the completer is still
/// fetching completions for the current directory. To avoid confusing
/// the UI, we retain the state of fetching completions for the
/// current directory before we actually descend into the chosen one.
///
/// The saved state consists of the collection of completions already
/// passed to the UI, an indication whether fetching data was already
/// finished, and an optional JoinHandle which is filled if fetching
/// was not done.
///
/// This is needed because we may need to return to that level via
/// ascend(), and we want to continue scanning directories exactly
/// from where we stopped. Even if collecting completions was
/// finished, we will have the completions ready for searching when we
/// return to this level.
pub struct FsCompleter {
    dir_path: path::PathBuf,
    all_completions: Vec<core::CompletionBox>,
    filtered_completions: Vec<core::CompletionBox>,
    query: String,
    fetching_thread: Option<BgThread>,
}

impl FsCompleter {
    pub fn new(dir_path: path::PathBuf) -> FsCompleter {
        let (request_send, request_recv) = mpsc::channel::<()>();
        let (response_send, response_recv) = mpsc::channel::<Option<Vec<core::CompletionBox>>>();
        let dir_path_clone = dir_path.clone();
        let thread = thread::spawn(
            move || fetching_thread_routine(dir_path_clone, request_recv, response_send)
        );
        let bg_thread = BgThread {
            thread: thread,
            request_send: request_send,
            response_recv: response_recv,
        };
       
        FsCompleter {
            dir_path: dir_path,
            all_completions: vec![],
            filtered_completions: vec![],
            query: String::new(),
            fetching_thread: Some(bg_thread),
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

impl core::Completer for FsCompleter {
    fn name(&self) -> String {
        "fs".to_owned()
    }

    fn completions(&self) -> &[core::CompletionBox] {
        self.filtered_completions.as_slice()
    }

    fn fetching_completions_finished(&self) -> bool {
        match self.fetching_thread {
            Some(_) => false,
            None    => true,
        }
    }

    fn fetch_completions(&mut self) {
        let bg_thread = self.fetching_thread.take();
        if let Some(t) = bg_thread {
            t.request_send.send(()).unwrap();
            let new_completions = t.response_recv.recv().unwrap();
            match new_completions {
                Some(completions) => {
                    let filtered_completions = self.filter_completions(&completions);
                    self.filtered_completions.extend(filtered_completions);
                    self.all_completions.extend(completions);
                    // We have 'taken' bg_thread out of the structure, but it turns
                    // out we have to restore it.
                    self.fetching_thread = Some(t);
                },
                None => {
                    t.thread.join().unwrap();
                }
            }
        }
    }

    fn set_query(&mut self, query: String) {
        self.query = query;
        self.filtered_completions = self.filter_completions(self.all_completions.as_slice());
    }

    fn descend(&self, completion: &core::Completion) -> Option<Box<core::Completer>> {
        let completion_any = completion.as_any();
        let fs_completion = completion_any.downcast_ref::<FsCompletion>().unwrap();
        match fs_completion.entry_type {
            FsEntryType::Directory => {
                let new_path = self.dir_path.join(fs_completion.relative_path.file_name().unwrap());
                Some(Box::new(FsCompleter::new(new_path)))
            },
            _ => None,
        }
    }

    fn ascend(&self) -> Option<Box<core::Completer>> {
        let current_path = self.dir_path.clone();
        if current_path.ends_with(path::Path::new(".")) {
            Some(Box::new(FsCompleter::new(path::PathBuf::from(".."))))
        } else if current_path.ends_with(path::Path::new("..")) {
            let mut new_path = current_path.join(path::Path::new(".."));
            if new_path.canonicalize().unwrap() == path::Path::new("/") {
                new_path = path::PathBuf::from("/");
            }
            Some(Box::new(FsCompleter::new(new_path)))
        } else {
            None
        }
    }
}
