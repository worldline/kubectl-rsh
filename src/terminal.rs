use std::{
    error::Error,
    os::fd::{AsFd, AsRawFd},
};

use kube::api::TerminalSize;
use nix::{
    libc::{VMIN, VTIME},
    sys::termios::{self, Termios},
};

// As defined in termios.h
#[allow(dead_code)]
pub struct Winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

const TIOCGWINSZ: u16 = 21523;

nix::ioctl_read_bad!(tcgetwinsize, TIOCGWINSZ, Winsize);

pub fn get_terminal_size<Fd: AsRawFd>(term_fd: Fd) -> Result<TerminalSize, Box<dyn Error>> {
    let mut ws = Winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    unsafe {
        let _ = tcgetwinsize(term_fd.as_raw_fd(), &mut ws);
    }

    Ok(TerminalSize {
        width: ws.ws_col,
        height: ws.ws_row,
    })
}

pub fn make_terminal_raw<Fd: AsFd>(term_fd: &Fd) -> Result<Termios, Box<dyn Error>> {
    let mut term_attr = termios::tcgetattr(&term_fd)?;
    let term_prev_attr = term_attr.clone();

    // Make the terminal "raw" - https://www.man7.org/linux/man-pages/man3/termios.3.html
    // Logging not recommended after this point, it will look messed up
    termios::cfmakeraw(&mut term_attr);
    term_attr.control_chars[VTIME] = 0;
    term_attr.control_chars[VMIN] = 1;
    match termios::tcsetattr(&term_fd, termios::SetArg::TCSAFLUSH, &term_attr) {
        Ok(_) => Ok(term_prev_attr),
        Err(e) => Err(format!("error making terminal raw: {}", e).into()),
    }
}

pub fn restore_term_attr<Fd: AsFd>(
    term_fd: Fd,
    terminal_attr: &termios::Termios,
) -> Result<(), Box<dyn Error>> {
    match termios::tcsetattr(term_fd, termios::SetArg::TCSAFLUSH, terminal_attr) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("error restoring terminal state: {}", e).into()),
    }
}
