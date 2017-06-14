use std::cmp;
use std::fs::File;
use std::io;
use std::io::Write;

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
    propositions: core::Completions,
    fetching_done: bool,
    view_offset: usize,
    selection: usize,
    query: String,
}

impl LevelViewState {
    pub fn new(completions_result: core::GetCompletionsResult) -> LevelViewState {
        let core::GetCompletionsResult(completions, fetching_done) = completions_result;
        LevelViewState {
            propositions: completions,
            fetching_done: fetching_done,
            view_offset: 0,
            selection: 0,
            query: "".to_string(),
        }
    }

    fn selected_completion(&self) -> &core::Completion {
        &*self.propositions[self.selection]
    }

    pub fn select_previous(&mut self) {
        self.selection = self.selection.saturating_sub(1);
        if self.selection < self.view_offset {
            self.view_offset = self.view_offset - 1;
        }
    }

    pub fn select_next(&mut self) {
        self.selection = cmp::min(self.selection + 1, self.propositions.len() - 1);
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
        self.selection = cmp::min(self.selection + CHOOSER_HEIGHT, self.propositions.len() - 1);
        if self.selection >= self.view_offset + CHOOSER_HEIGHT {
            self.view_offset = self.selection.saturating_sub(CHOOSER_HEIGHT - 1);
        }
    }

    pub fn select_first(&mut self) {
        self.selection = 0;
        self.view_offset = 0;
    }
    
    pub fn select_last(&mut self) {
        self.selection = self.propositions.len() - 1;
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
    pub fn new(completer: &mut core::Completer) -> ViewState {
        ViewState {
            levels_stack: vec![LevelViewState::new(completer.get_completions())],
        }
    }

    pub fn top(&self) -> &LevelViewState {
        self.levels_stack.last().unwrap()
    }

    fn selected_completion(&self) -> &core::Completion {
        self.top().selected_completion()
    }

    pub fn get_selected_result(&self) -> String {
        self.top().selected_completion().result_string()
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
        self.levels_stack.last_mut().unwrap().query.pop();
    }

    pub fn query_append(&mut self, ch: char) {
        self.levels_stack.last_mut().unwrap().query.push(ch);
    }

    pub fn query_set(&mut self, query: &str) {
        self.levels_stack.last_mut().unwrap().query_set(query);
    }

    pub fn query(&self) -> String {
        self.top().query().clone()
    }

    fn descend(&mut self, completer: &mut core::Completer) {
        self.levels_stack.push(LevelViewState::new(completer.get_completions()));
    }

    fn is_descended(&self) -> bool {
        self.levels_stack.len() > 1
    }

    fn ascend(&mut self) {
        self.levels_stack.pop();
    }

    fn switch_base(&mut self, completer: &mut core::Completer) {
        self.levels_stack[0] = LevelViewState::new(completer.get_completions());
    }
}

fn print_state(term: &mut File, state: &LevelViewState) -> io::Result<()> {
    let off = state.view_offset;
    let prompt = "  Search: ";
    let status_string = format!("STATUS {:?} {:?} ql {:?}", off, state.selection, state.query.len());
    let term_cols = 80 as usize;

    writeln!(term, "{}{}{}{}{:>sw$}", termion::cursor::Left(100),
             clear::CurrentLine, prompt, state.query, status_string,
             sw = term_cols - prompt.len() - state.query.len())?;

    let end_offset = cmp::min(off + CHOOSER_HEIGHT, state.propositions.len());
    for (i, p) in state.propositions[off .. end_offset].iter().enumerate() {
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

pub fn get_completion(mut line: String, completer: &mut core::Completer)
                      -> io::Result<(String, i16)> {
    let mut term = termion::get_tty()?;
    let mut state = ViewState::new(completer);

    let original_query = get_initial_query(line.as_str());
    state.query_set(original_query.as_str());

    let original_terminal_state = terminal::prepare()?;
    write!(term, "{}", termion::cursor::Right(30))?;

    print_state(&mut term, state.top()).unwrap();

    let mut result = String::new();

    for key_result in io::stdin().keys() {
        match key_result.unwrap() {
            Up         => state.select_previous(),
            Down       => state.select_next(),
            PageUp     => state.previous_page(),
            PageDown   => state.next_page(),
            Home       => state.select_first(),
            End        => state.select_last(),

            Left       => {
                if completer.can_ascend() {
                    completer.ascend();
                    if state.is_descended() {
                        state.ascend();
                    } else {
                        state.switch_base(completer);
                    }
                }
            }
            Right      => {
                if completer.can_descend(state.selected_completion()) {
                    completer.descend(state.selected_completion());
                    state.descend(completer);
                }
            }

            Char('\n') => { result = state.get_selected_result(); break },
            Ctrl('c')  => { result = original_query.clone(); break },
            Char(c)    => state.query_append(c),
            Backspace  => state.query_backspace(),

            _ => {},
        }
        print_state(&mut term, state.top())?;
    }

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
