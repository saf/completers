use std::io;
use std::os::unix::io::RawFd;

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
