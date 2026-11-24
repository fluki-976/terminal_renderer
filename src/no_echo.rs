use std::io;
use std::os::fd::RawFd;

use termios::{ECHO, TCSAFLUSH, TCSANOW, Termios, tcsetattr};

/// Stops typed keys echoing to the screen, input stays line buffered
///
/// The settings saved on [`NoEcho::enable`] are restored when this is dropped,
/// so the user's terminal comes back even after a panic or an early `?`
pub(crate) struct NoEcho {
    fd: RawFd,
    original: Termios,
}

impl NoEcho {
    /// Turns echo off on `fd` until the returned guard is dropped
    pub(crate) fn enable(fd: RawFd) -> io::Result<Self> {
        let original = Termios::from_fd(fd)?;
        let mut no_echo = original;
        /* Only disables echo, leave rest as default */
        no_echo.c_lflag &= !ECHO;
        tcsetattr(fd, TCSANOW, &no_echo)?;
        Ok(Self { fd, original })
    }
}

impl Drop for NoEcho {
    fn drop(&mut self) {
        // Flush discards buffered unused stdin so the terminal dosn`t run
        // it on program exit
        // Nothing useful to do if restoring fails while dropping
        let _ = tcsetattr(self.fd, TCSAFLUSH, &self.original);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::ptr::{null, null_mut};

    /// Opens a pseudo-terminal pair as (master, slave)
    /// Both of the pair must remain in scope else pty hangs
    fn open_pty() -> (OwnedFd, OwnedFd) {
        let (mut master, mut slave) = (0, 0);
        let rc = unsafe { libc::openpty(&mut master, &mut slave, null_mut(), null(), null()) };
        assert_eq!(rc, 0, "openpty failed");
        unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) }
    }

    /// True if fd has echo On
    fn echo_on(fd: RawFd) -> bool {
        Termios::from_fd(fd).unwrap().c_lflag & ECHO != 0
    }

    /// Bytes typed into the terminal but not yet read
    fn queued_input(fd: RawFd) -> i32 {
        let mut n = 0;
        unsafe { libc::ioctl(fd, libc::FIONREAD, &mut n) };
        n
    }

    #[test]
    fn enable_turns_echo_off() {
        let (_master, slave) = open_pty();
        let _guard = NoEcho::enable(slave.as_raw_fd()).unwrap();
        assert!(!echo_on(slave.as_raw_fd()));
    }

    #[test]
    fn drop_turns_echo_back_on() {
        let (_master, slave) = open_pty();
        let guard = NoEcho::enable(slave.as_raw_fd()).unwrap();
        // Force echo off so this test does not lean on enable working
        let mut attrs = Termios::from_fd(slave.as_raw_fd()).unwrap();
        attrs.c_lflag &= !ECHO;
        tcsetattr(slave.as_raw_fd(), TCSANOW, &attrs).unwrap();

        drop(guard);
        assert!(echo_on(slave.as_raw_fd()));
    }

    #[test]
    fn drop_discards_unread_input() {
        let (master, slave) = open_pty();
        let guard = NoEcho::enable(slave.as_raw_fd()).unwrap();
        // Keys typed while the program runs, left for the shell if not discarded
        let typed = b"touch /tmp/leaked\n";
        unsafe { libc::write(master.as_raw_fd(), typed.as_ptr().cast(), typed.len()) };
        std::thread::sleep(std::time::Duration::from_millis(50)); // pty delivers async

        drop(guard);
        assert_eq!(queued_input(slave.as_raw_fd()), 0);
    }
}
