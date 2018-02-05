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

    fn selected_completion(&self) -> Option<&core::Completion> {
        let completions = self.completer.completions();
        if completions.is_empty() {
            None
        } else {
            Some(&*completions[self.selection])
        }
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

    fn update_query(&mut self) {
        self.selection = 0;
        self.view_offset = 0;
        self.completer.set_query(self.query.clone());
    }

    pub fn query_backspace(&mut self) {
        self.query.pop();
        self.update_query()
    }

    pub fn query_append(&mut self, ch: char) {
        self.query.push(ch);
        self.update_query()
    }

    pub fn query_set(&mut self, query: &str) {
        self.query = query.to_string();
        self.update_query()
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

    pub fn get_selected_result(&self) -> Option<String> {
        self.top().selected_completion().map(|c| c.result_string())
    }

    pub fn select_previous(&mut self) {
        self.top_mut().select_previous();
    }

    pub fn select_next(&mut self) {
        self.top_mut().select_next();
    }

    pub fn previous_page(&mut self) {
        self.top_mut().previous_page();
    }

    pub fn next_page(&mut self) {
        self.top_mut().next_page();
    }

    pub fn select_first(&mut self) {
        self.top_mut().select_first();
    }

    pub fn select_last(&mut self) {
        self.top_mut().select_last();
    }

    pub fn query_backspace(&mut self) {
        self.top_mut().query_backspace();
    }

    pub fn query_append(&mut self, ch: char) {
        self.top_mut().query_append(ch);
    }

    pub fn query_set(&mut self, query: &str) {
        self.top_mut().query_set(query);
    }

    pub fn query(&self) -> String {
        self.top().query()
    }

    pub fn fetch_completions(&mut self) {
        self.top_mut().completer.fetch_completions();
    }

    fn descend(&mut self) {
        if self.top().selected_completion().is_none() {
            ()
        } else {
            let new_completer_or_nothing =
                self.top().completer.descend(self.top().selected_completion().unwrap());
            if let Some(mut new_completer) = new_completer_or_nothing {
                new_completer.fetch_completions();
                self.levels_stack.push(LevelViewState::new(new_completer));
            }
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

struct State {
    /// The collection of tabs (completer stacks).
    tabs: Vec<ViewState>,

    /// The index within `tabs` which is currently selected.
    selection: usize,
}

impl State {
    fn new(completers: Vec<Box<core::Completer>>) -> State {
        let mut tabs = vec![];
        for c in completers {
            tabs.push(ViewState::new(c));
        }
        State {
            tabs: tabs,
            selection: 0,
        }
    }

    fn current_tab(&self) -> &ViewState {
        &self.tabs[self.selection]
    }

    fn current_tab_mut(&mut self) -> &mut ViewState {
        &mut self.tabs[self.selection]
    }

    pub fn get_selected_result(&self) -> Option<String> {
        self.current_tab().get_selected_result()
    }

    pub fn select_previous(&mut self) {
        self.current_tab_mut().select_previous();
    }

    pub fn select_next(&mut self) {
        self.current_tab_mut().select_next();
    }

    pub fn previous_page(&mut self) {
        self.current_tab_mut().previous_page();
    }

    pub fn next_page(&mut self) {
        self.current_tab_mut().next_page();
    }

    pub fn select_first(&mut self) {
        self.current_tab_mut().select_first();
    }

    pub fn select_last(&mut self) {
        self.current_tab_mut().select_last();
    }

    pub fn query_backspace(&mut self) {
        self.current_tab_mut().query_backspace();
    }

    pub fn query_append(&mut self, ch: char) {
        self.current_tab_mut().query_append(ch);
    }

    pub fn query_set(&mut self, query: &str) {
        self.current_tab_mut().query_set(query);
    }

    pub fn query(&self) -> String {
        self.current_tab().query().clone()
    }

    pub fn descend(&mut self) {
        self.current_tab_mut().descend()
    }

    pub fn ascend(&mut self) {
        self.current_tab_mut().ascend()
    }

    fn next_tab(&mut self) {
        self.selection = (self.selection + 1) % self.tabs.len();
    }

    fn start_fetching_completions(&mut self) {
        for t in &mut self.tabs {
            t.fetch_completions();
        }
    }

    fn fetch_completions(&mut self) {
        self.current_tab_mut().fetch_completions();
    }
}

fn print_state(term: &mut File, state: &State) -> io::Result<()> {
    let completer_stack = &state.tabs[state.selection];
    let level_state = completer_stack.top();
    let off = level_state.view_offset;
    let prompt = "  Search: ";
    let completions = level_state.completer.completions();
    let status_string = format!("[{} {}-{}/{}]", level_state.completer.name(), off + 1,
                                cmp::min(off + CHOOSER_HEIGHT + 1, completions.len()),
                                completions.len());
    let term_cols = terminal::get_width(term).unwrap() as usize;

    writeln!(term, "{}{}{}{}{:>sw$}", termion::cursor::Left(100),
             clear::CurrentLine, prompt, state.query(), status_string,
             sw = term_cols - prompt.len() - level_state.query.len())?;

    let end_offset = cmp::min(off + CHOOSER_HEIGHT, completions.len());
    for (i, p) in completions[off .. end_offset].iter().enumerate() {
        let completion_string = p.display_string();
        let displayed_length = cmp::min(completion_string.len(), term_cols - 2);
        let displayed_completion = &(completion_string)[..displayed_length];
        if off + i == level_state.selection {
            writeln!(term, "{}{}{}{}{}{}",
                     clear::CurrentLine, Bg(Black), Fg(White),
                     displayed_completion,
                     Fg(Reset), Bg(Reset))?;
        } else {
            writeln!(term, "{}{}", clear::CurrentLine, displayed_completion)?;
        }
    }

    for _ in end_offset .. off + CHOOSER_HEIGHT {
        writeln!(term, "{}", clear::CurrentLine)?;
    }

    write!(term, "{}{}",
           termion::cursor::Up((CHOOSER_HEIGHT + 1) as u16),
           termion::cursor::Right((prompt.len() + level_state.query.len()) as u16))?;
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

pub fn get_completion(mut line: String, completers: Vec<Box<core::Completer>>)
                      -> io::Result<(String, i16)> {
    let mut term = termion::get_tty()?;
    let mut state = State::new(completers);

    let original_query = get_initial_query(line.as_str());
    state.current_tab_mut().query_set(original_query.as_str());

    let original_terminal_state = terminal::prepare()?;
    write!(term, "{}", termion::cursor::Right(30))?;

    state.start_fetching_completions();
    print_state(&mut term, &state).unwrap();

    let result;

    let (key_sender, key_receiver) = mpsc::channel::<termion::event::Key>();
    let (req_sender, req_receiver) = mpsc::channel::<()>();
    let key_reader_thread = thread::spawn(move || key_reader_thread_routine(req_receiver, key_sender));
    let mut req_sender = Some(req_sender);

    req_sender.as_ref().unwrap().send(()).unwrap();
    loop {
        let key_or_nothing;
        if !state.current_tab().top().completer.fetching_completions_finished() {
            key_or_nothing = key_receiver.recv_timeout(time::Duration::from_millis(10)).ok();
            state.fetch_completions();
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

                Char('\n') => {
                    if let Some(r) = state.get_selected_result() {
                        result = r ; break
                    }
                },
                Ctrl('c')  => { result = original_query.clone(); break },
                Char('\t') => state.next_tab(),
                Char(c)    => state.query_append(c),
                Backspace  => state.query_backspace(),

                _ => {},
            };
            // We are going to loop again, so we send a request to get another input key.
            req_sender.as_ref().unwrap().send(()).unwrap();
        }
        print_state(&mut term, &state)?;
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
