///! Module representing a canvas for writing to and drawing on the terminal.
///
/// Using a canvas from this module is different from using ncurses in that
/// a canvas does not fill the entire terminal screen (does not use the
/// alternate screen feature), but allows modifying a portion of the terminal
/// screen within the current window below the current command line.

use std::fs;
use std::io;
use std::io::Write;

use termion;

use super::terminal;

pub struct TermCanvas {
    term: fs::File,
    start_row: usize,
    start_col: usize,
    width: usize,
    height: usize,
}

impl TermCanvas {
    pub fn new(mut term: fs::File, height: usize) -> io::Result<TermCanvas> {
        let (term_cols, _term_rows) = terminal::get_dimensions()?;
        for _ in 0..height {
            term.write(b"\n")?;
        }
        write!(term, "{}", termion::cursor::Up(height as u16))?;
        let (_, start_row) = terminal::get_cursor_position()?;
        Result::Ok(TermCanvas {
            term: term,
            start_row: start_row as usize - 1,
            start_col: 0,
            width: term_cols,
            height: height,
        })
    }

    pub fn move_to(&mut self, row: usize, col: usize) -> io::Result<()> {
        // TODO Add bounds checking.
        write!(self.term, "{}", termion::cursor::Goto(
            (col + self.start_col + 1) as u16,
            (row + self.start_row + 1) as u16)
        )?;
        Result::Ok(())
    }

    pub fn clear(&mut self) -> io::Result<()> {
        for i in 0..self.height {
            self.move_to(i, 0)?;
            write!(self.term, "{}", termion::clear::CurrentLine)?;
        }
        self.move_to(0, 0)?;
        Result::Ok(())
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }
}

impl Write for TermCanvas {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.term.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.term.flush()
    }
}