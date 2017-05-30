use std::cmp;
use std::fs::File;
use std::io;
use std::io::Write;

use ctrlc;

use termion;
use termion::clear;
use termion::input::TermRead;
use termion::color::*;
use termion::event::Key::*;
use termion::cursor::DetectCursorPos;

use termsize;

// TODO: make this value configurable.
const CHOOSER_HEIGHT: usize = 10;

struct ViewState<'a> {
    propositions: &'a Vec<&'a str>,
    view_offset: usize,
    selection: usize,
}

impl<'a> ViewState<'a> {
    pub fn new(propositions: &'a Vec<&'a str>) -> ViewState<'a> {
        ViewState {
            propositions: propositions,
            view_offset: 0,
            selection: 0,
        }
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
}

fn print_state(term: &mut File, state: &ViewState) -> io::Result<()> {
    let off = state.view_offset;
    let prompt = "  Search: ";
    let mut query = "";
    let status_string = format!("STATUS {:?} {:?}", off, state.selection);
    let term_cols = 80 as usize;

    writeln!(term, "{}{}{}{}{:>sw$}", termion::cursor::Left((prompt.len() + query.len()) as u16),
             clear::CurrentLine, prompt, query, status_string,
             sw = term_cols - prompt.len() - query.len())?;
    
    for (i, p) in state.propositions[off .. off + CHOOSER_HEIGHT].iter().enumerate() {
        if off + i == state.selection {
            writeln!(term, "{}{}{}{}{}{}",
                     clear::CurrentLine, Bg(Black), Fg(White), p, Fg(Reset), Bg(Reset))?;
        } else {
            writeln!(term, "{}{}", clear::CurrentLine, p)?;
        }
    }
    
    write!(term, "{}{}",
           termion::cursor::Up((CHOOSER_HEIGHT + 1) as u16),
           termion::cursor::Right((prompt.len() + query.len()) as u16))?;
    return Result::Ok(());
}

pub fn get_completion(line: &str, propositions: &Vec<&str>) -> io::Result<String> {
    let mut term = termion::get_tty()?;
    let mut state = ViewState::new(propositions);

    print_state(&mut term, &state)?;
    let mut result = String::new();

    for key_result in io::stdin().keys() {
        match key_result.unwrap() {
            Up => state.select_previous(),
            Down => state.select_next(),
            PageUp => state.previous_page(),
            PageDown => state.next_page(),
            Home => state.select_first(),
            End => state.select_last(),
            Char('\n') => { result = propositions[state.selection].to_string(); break },
            _ => {},
        }
        print_state(&mut term, &state)?;
    }

    for _ in 0..CHOOSER_HEIGHT {
        write!(term, "{}{}", clear::CurrentLine, termion::cursor::Down(1));
    }
    write!(term, "{}{}", termion::cursor::Left(100),
           termion::cursor::Up(CHOOSER_HEIGHT as u16 - 1));

    return Result::Ok(result);
}

pub fn clear() -> io::Result<()> {
    let mut term = termion::get_tty()?;
    write!(term, "{}", termion::cursor::Left(100));
    write!(term, "{}", termion::cursor::Up(1));
    return Result::Ok(());
}
