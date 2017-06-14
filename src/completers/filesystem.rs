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
    thread: thread::JoinHandle<()>,
    request_send: mpsc::Sender<()>,
    response_recv: mpsc::Receiver<core::GetCompletionsResult>,
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
struct FsCompleterLevelState {
    pub dir_path: path::PathBuf,
    pub all_completions: core::Completions,
    pub fetching_thread: Option<BgThread>,
}

fn directory_bfs(queue: &mut VecDeque<DirectoryQueueEntry>) -> core::Completions {
    let queue_entry = queue.pop_front();
    if let None = queue_entry {
        return core::Completions::new();
    }
    let DirectoryQueueEntry(dir_path, depth) = queue_entry.unwrap();
    let mut completions = core::Completions::new();
    let read_dir_result = fs::read_dir(&dir_path);
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

        completions.push(Arc::new(FsCompletion {
            relative_path: path,
            entry_type: entry_type,
        }));
    }
    completions.sort_by_key(|b| b.result_string());
    completions
}

fn fetching_thread_routine(dir_path: path::PathBuf, request_recv: mpsc::Receiver<()>,
                           response_send: mpsc::Sender<core::GetCompletionsResult>) {
    let mut dir_queue: VecDeque<DirectoryQueueEntry> = VecDeque::new();
    dir_queue.push_back(DirectoryQueueEntry(dir_path, 0));
    let mut completions = core::Completions::new();
    while !dir_queue.is_empty() {
        completions.extend(directory_bfs(&mut dir_queue));
        match request_recv.try_recv() {
            Result::Ok(_) => {
                let result = core::GetCompletionsResult(completions, false);
                response_send.send(result).unwrap();
                completions = core::Completions::new();
            },
            Result::Err(mpsc::TryRecvError::Empty) => {},
            Result::Err(mpsc::TryRecvError::Disconnected) => {
                return;
            }
        }
    }
    match request_recv.recv() {
        Result::Ok(_) => {
            let result = core::GetCompletionsResult(completions, true);
            response_send.send(result).unwrap();
        },
        Result::Err(_) => {
            return;
        }
    }
}

impl FsCompleterLevelState {
    fn new(dir_path: path::PathBuf) -> FsCompleterLevelState {
        let (request_send, request_recv) = mpsc::channel::<()>();
        let (response_send, response_recv) = mpsc::channel::<core::GetCompletionsResult>();
        let dir_path_clone = dir_path.clone();
        let thread = thread::spawn(
            move || fetching_thread_routine(dir_path_clone, request_recv, response_send)
        );
        let bg_thread = BgThread {
            thread: thread,
            request_send: request_send,
            response_recv: response_recv,
        };
       
        FsCompleterLevelState {
            dir_path: dir_path,
            all_completions: core::Completions::new(),
            fetching_thread: Some(bg_thread),
        }
    }

    fn get_completions(&mut self) -> core::GetCompletionsResult {
        let bg_thread = self.fetching_thread.take();
        if let Some(t) = bg_thread {
            t.request_send.send(()).unwrap();
            let completions_result = t.response_recv.recv().unwrap();
            {
                let core::GetCompletionsResult(ref completions, is_finished) = completions_result;
                self.all_completions.extend(completions.clone());
                if is_finished {
                    t.thread.join().unwrap();
                } else {
                    // We have 'taken' bg_thread out of the structure, but it turns
                    // out we have to restore it.
                    self.fetching_thread = Some(t)
                }
            }
            completions_result
        } else {
            core::GetCompletionsResult(vec![], true)
        }
    }
}

pub struct FsCompleter {
    level_states: Vec<FsCompleterLevelState>,
}

impl FsCompleter {
    pub fn new() -> FsCompleter {
        let level_state = FsCompleterLevelState::new(path::PathBuf::from("."));
        FsCompleter {
            level_states: vec![level_state],
        }
    }
}

impl core::Completer for FsCompleter {
    fn get_completions(&mut self) -> core::GetCompletionsResult {
        self.level_states.last_mut().unwrap().get_completions()
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
        let descend_dir = fs_completion.relative_path.file_name().unwrap();
        let new_path = self.level_states.last().unwrap().dir_path.join(descend_dir);
        self.level_states.push(FsCompleterLevelState::new(new_path));
    }

    fn can_ascend(&self) -> bool {
        self.level_states.last().unwrap().dir_path != path::Path::new("/")
    }

    fn ascend(&mut self) {
        let current_path = self.level_states.last().unwrap().dir_path.clone();
        if current_path.ends_with(path::Path::new(".")) {
            self.level_states[0] = FsCompleterLevelState::new(path::PathBuf::from(".."));
        } else if current_path.ends_with(path::Path::new("..")) {
            let mut new_path = current_path.join(path::Path::new(".."));
            if new_path.canonicalize().unwrap() == path::Path::new("/") {
                new_path = path::PathBuf::from("/");
            }
            self.level_states[0] = FsCompleterLevelState::new(new_path);
        } else {
            self.level_states.pop();
        }
    }
}
