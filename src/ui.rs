use std::cmp;
use std::fs::File;
use std::io;
use std::io::Write;
use std::sync::mpsc;
use std::time;
use std::thread;

use termion;
use termion::clear;
use termion::input::TermRead;
use termion::color::*;
use termion::event::Key::*;

use super::core;
use super::terminal;

// TODO: make this value configurable.
const CHOOSER_HEIGHT: usize = 10;

const WORD_BOUNDARIES: &'static [char] = &[' ', '(', ')', ':', '`'];

struct LevelViewState {
    completer: Box<core::Completer>,
    view_offset: usize,
    selection: usize,
    query: String,
}

impl LevelViewState {
    pub fn new(completer: Box<core::Completer>) -> LevelViewState {
        LevelViewState {
            completer: completer,
            view_offset: 0,
            selection: 0,
            query: "".to_string(),
        }
    }

    fn selected_completion(&self) -> &core::Completion {
        let completions = self.completer.completions();
        &*completions[self.selection]
    }

    pub fn select_previous(&mut self) {
        self.selection = self.selection.saturating_sub(1);
        if self.selection < self.view_offset {
            self.view_offset = self.view_offset - 1;
        }
    }

    pub fn select_next(&mut self) {
        let completions_count = self.completer.completions().len();
        self.selection = cmp::min(self.selection + 1, completions_count.saturating_sub(1));
        if self.selection >= self.view_offset + CHOOSER_HEIGHT {
            self.view_offset = self.view_offset + 1;
        }
    }

    pub fn previous_page(&mut self) {
        self.selection = self.selection.saturating_sub(CHOOSER_HEIGHT);
        if self.selection < self.view_offset {
            self.view_offset = self.selection;
        }
    }

    pub fn next_page(&mut self) {
        let completions_count = self.completer.completions().len();
        self.selection = cmp::min(self.selection + CHOOSER_HEIGHT, completions_count - 1);
        if self.selection >= self.view_offset + CHOOSER_HEIGHT {
            self.view_offset = self.selection.saturating_sub(CHOOSER_HEIGHT - 1);
        }
    }

    pub fn select_first(&mut self) {
        self.selection = 0;
        self.view_offset = 0;
    }

    pub fn select_last(&mut self) {
        let completions_count = self.completer.completions().len();
        self.selection = completions_count - 1;
        self.view_offset = self.selection.saturating_sub(CHOOSER_HEIGHT - 1);
    }

    pub fn query_backspace(&mut self) {
        self.query.pop();
    }

    pub fn query_append(&mut self, ch: char) {
        self.query.push(ch);
    }

    pub fn query_set(&mut self, query: &str) {
        self.query = query.to_string();
    }

    pub fn query(&self) -> String {
        self.query.clone()
    }
}

struct ViewState {
    levels_stack: Vec<LevelViewState>,
}

impl ViewState {
    pub fn new(completer: Box<core::Completer>) -> ViewState {
        ViewState {
            levels_stack: vec![LevelViewState::new(completer)],
        }
    }

    pub fn top(&self) -> &LevelViewState {
        self.levels_stack.last().unwrap()
    }

    pub fn top_mut(&mut self) -> &mut LevelViewState {
        self.levels_stack.last_mut().unwrap()
    }

    pub fn get_selected_result(&mut self) -> String {
        self.top_mut().selected_completion().result_string()
    }

    pub fn select_previous(&mut self) {
        self.levels_stack.last_mut().unwrap().select_previous();
    }

    pub fn select_next(&mut self) {
        self.levels_stack.last_mut().unwrap().select_next();
    }

    pub fn previous_page(&mut self) {
        self.levels_stack.last_mut().unwrap().previous_page();
    }

    pub fn next_page(&mut self) {
        self.levels_stack.last_mut().unwrap().next_page();
    }

    pub fn select_first(&mut self) {
        self.levels_stack.last_mut().unwrap().select_first();
    }

    pub fn select_last(&mut self) {
        self.levels_stack.last_mut().unwrap().select_last();
    }

    pub fn query_backspace(&mut self) {
        self.levels_stack.last_mut().unwrap().query_backspace();
    }

    pub fn query_append(&mut self, ch: char) {
        self.levels_stack.last_mut().unwrap().query_append(ch);
    }

    pub fn query_set(&mut self, query: &str) {
        self.levels_stack.last_mut().unwrap().query_set(query);
    }

    pub fn query(&self) -> String {
        self.top().query().clone()
    }

    fn descend(&mut self) {
        let new_completer_or_nothing = self.top().completer.descend(self.top().selected_completion());
        if let Some(mut new_completer) = new_completer_or_nothing {
            new_completer.fetch_completions();
            self.levels_stack.push(LevelViewState::new(new_completer));
        }
    }

    fn ascend(&mut self) {
        if self.levels_stack.len() == 1 {
            if let Some(mut new_completer) = self.top().completer.ascend() {
                new_completer.fetch_completions();
                self.levels_stack[0] = LevelViewState::new(new_completer);
            }
        } else {
            self.levels_stack.pop();
        }
    }
}

fn print_state(term: &mut File, state: &LevelViewState) -> io::Result<()> {
    let off = state.view_offset;
    let prompt = "  Search: ";
    let completions = state.completer.completions();
    let status_string = format!("[{}-{}/{}]", off + 1,
                                cmp::min(off + CHOOSER_HEIGHT + 1, completions.len()),
                                completions.len());
    let term_cols = 80 as usize;

    writeln!(term, "{}{}{}{}{:>sw$}", termion::cursor::Left(100),
             clear::CurrentLine, prompt, state.query, status_string,
             sw = term_cols - prompt.len() - state.query.len())?;

    let end_offset = cmp::min(off + CHOOSER_HEIGHT, completions.len());
    for (i, p) in completions[off .. end_offset].iter().enumerate() {
        if off + i == state.selection {
            writeln!(term, "{}{}{}{}{}{}",
                     clear::CurrentLine, Bg(Black), Fg(White), p.display_string(),
                     Fg(Reset), Bg(Reset))?;
        } else {
            writeln!(term, "{}{}", clear::CurrentLine, p.display_string())?;
        }
    }

    for _ in end_offset .. off + CHOOSER_HEIGHT {
        writeln!(term, "{}", clear::CurrentLine)?;
    }

    write!(term, "{}{}",
           termion::cursor::Up((CHOOSER_HEIGHT + 1) as u16),
           termion::cursor::Right((prompt.len() + state.query.len()) as u16))?;
    return Result::Ok(());
}

pub fn get_initial_query(line: &str) -> String {
    let line_length = line.len();
    let last_word_boundary = line.rfind(WORD_BOUNDARIES);
    let word_index = match last_word_boundary {
        None => 0,
        Some(index) => index + 1,
    };

    if word_index >= line_length {
        "".to_string()
    } else {
        line[word_index..].to_string()
    }
}

fn key_reader_thread_routine(req_receiver: mpsc::Receiver<()>,
                             key_sender: mpsc::Sender<termion::event::Key>) {
    let mut keys = io::stdin().keys();
    while let Result::Ok(()) = req_receiver.recv() {
        if let Some(Result::Ok(key)) = keys.next() {
            let result = key_sender.send(key);
            if result.is_err() {
                break;
            }
        } else {
            break;
        }
    }
}

pub fn get_completion(mut line: String, completer: Box<core::Completer>)
                      -> io::Result<(String, i16)> {
    let mut term = termion::get_tty()?;
    let mut state = ViewState::new(completer);

    let original_query = get_initial_query(line.as_str());
    state.query_set(original_query.as_str());

    let original_terminal_state = terminal::prepare()?;
    write!(term, "{}", termion::cursor::Right(30))?;

    state.top_mut().completer.fetch_completions();
    print_state(&mut term, state.top()).unwrap();

    let result;

    let (key_sender, key_receiver) = mpsc::channel::<termion::event::Key>();
    let (req_sender, req_receiver) = mpsc::channel::<()>();
    let key_reader_thread = thread::spawn(move || key_reader_thread_routine(req_receiver, key_sender));
    let mut req_sender = Some(req_sender);

    req_sender.as_ref().unwrap().send(()).unwrap();
    loop {
        let key_or_nothing;
        if !state.top().completer.fetching_completions_finished() {
            key_or_nothing = key_receiver.recv_timeout(time::Duration::from_millis(10)).ok();
            state.top_mut().completer.fetch_completions();
        } else {
            key_or_nothing = key_receiver.recv().ok();
        }

        if let Some(key) = key_or_nothing {
            match key {
                Up         => state.select_previous(),
                Down       => state.select_next(),
                PageUp     => state.previous_page(),
                PageDown   => state.next_page(),
                Home       => state.select_first(),
                End        => state.select_last(),

                Left       => state.ascend(),
                Right      => state.descend(),

                Char('\n') => { result = state.get_selected_result(); break },
                Ctrl('c')  => { result = original_query.clone(); break },
                Char(c)    => state.query_append(c),
                Backspace  => state.query_backspace(),

                _ => {},
            };
            // We are going to loop again, so we send a request to get another input key.
            req_sender.as_ref().unwrap().send(()).unwrap();
        }
        print_state(&mut term, state.top())?;
    }

    req_sender.take();
    key_reader_thread.join().unwrap();

    clear()?;
    terminal::restore(original_terminal_state)?;

    let line_length = line.len();
    let original_length = original_query.len();
    let new_length = result.len();
    line.truncate(line_length - original_length);
    line.push_str(&result);
    return Result::Ok((line, (new_length - original_length) as i16));
}

pub fn clear() -> io::Result<()> {
    let mut term = termion::get_tty()?;
    for _ in 0..(CHOOSER_HEIGHT + 1) {
        write!(term, "{}{}", clear::CurrentLine, termion::cursor::Down(1))?;
    }
    write!(term, "{}{}", termion::cursor::Left(100),
           termion::cursor::Up((CHOOSER_HEIGHT + 1) as u16))?;
    return Result::Ok(());
}
