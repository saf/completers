use std::fs;
use std::io;
use std::os::raw::c_ushort;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::RawFd;

use libc;
use termios;

const INPUT_FD: RawFd = 0;

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

//  The following code is adapted from the 'terminal_size' crate by
//  Andrew Chin:
// 
//    https://github.com/eminence/terminal-size
//
//  The original code had to be modified to accept an arbitrary File
//  rather than use STDOUT. Our init.sh redirects output from this
//  program to a pipe, so our STDOUT is not connected to a terminal,
//  making requests for terminal size fail.

struct WinSize {
    ws_row: c_ushort,
    ws_col: c_ushort,
    ws_xpixel: c_ushort,
    ws_ypixel: c_ushort
}

/// Returns the size of the terminal, represented as a File object.
///
/// If STDOUT is not a tty, returns `None`
pub fn get_width(term: &fs::File) -> Option<u16> {
    let raw_fd = term.as_raw_fd();
    let is_tty: bool = unsafe{
        libc::isatty(raw_fd) == 1
    };

    if !is_tty { return None; }

    let cols = unsafe {
        let mut winsize = WinSize{ws_row: 0, ws_col: 0, ws_xpixel: 0, ws_ypixel: 0};
        libc::ioctl(raw_fd, libc::TIOCGWINSZ, &mut winsize);
        if winsize.ws_col > 0 {
            winsize.ws_col
        } else {
            0
        }
    };

    if cols > 0 {
        Some(cols)
    } else {
        None
    }
}
