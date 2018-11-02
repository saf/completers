use std::io;
use std::os;

use termios;

use term_size;
use term_cursor;

const INPUT_FD: os::unix::io::RawFd = 0;

pub fn prepare() -> io::Result<termios::Termios> {
    use termios::*;
    let original_term_settings = Termios::from_fd(INPUT_FD)?;

    let mut term_settings = original_term_settings;
    term_settings.c_lflag &= !(ISIG);
    tcsetattr(INPUT_FD, TCSANOW, &term_settings)?;
    return Result::Ok(original_term_settings);
}

pub fn restore(settings: termios::Termios) -> io::Result<()> {
    use termios::*;
    tcdrain(INPUT_FD)?;
    tcsetattr(INPUT_FD, TCSADRAIN, &settings)?;
    return Result::Ok(());
}

/// Returns the size of the terminal, in the form of
/// a tuple of (columns, rows).
///
/// If STDOUT is not a tty, returns `io::Error`
pub fn get_dimensions() -> io::Result<(usize, usize)> {
    term_size::dimensions().ok_or(
        io::Error::new(io::ErrorKind::Other,
                       "failed to fetch terminal dimensions")
    )
}

/// Returns the cursor position within the terminal, in the form of a
/// tuple of (row, column).
pub fn get_cursor_position() -> io::Result<(i32, i32)> {
    term_cursor::get_pos().or(
        Result::Err(io::Error::new(io::ErrorKind::Other, "failed to fetch cursor position"))
    )
}