#[rustfmt::skip]
use std::{
    io::{self, Write, Error, ErrorKind}, 
    process::{Command, Stdio}, 
    marker::PhantomData,
};

use crate::pane;

/*****************************************************************************/
/*                                 Screen                                    */
/*****************************************************************************/
#[derive(Default)]
pub struct Screen {
    /// Total Rows in terminal window
    pub rows: usize,
    /// Total Columns in terminal window
    pub columns: usize,
    /// Track each pane added to the screen
    pub panes: Vec<Box<dyn pane::Pane>>,
}

impl Screen {
    /// New instance of empty screen, set with the current
    /// size of the terminal window
    ///
    /// Multiple screens can be used but its up to the user
    /// to handle clearing previous screen before switching
    pub fn new() -> io::Result<Self> {
        let mut screen = Self::default();
        screen.read_terminal_size()?;
        Ok(screen)
    }

    /**********************************************************
     *                       Public                           *
     **********************************************************/
    /// Adds a pane to the screen and returns the handle used to reach it
    ///
    /// Panes are drawn in the order they are added. The returned
    /// [`PaneId`](crate::PaneId) remembers the pane's type, so
    /// [`Screen::pane`] hands back that same type
    pub fn add_pane<P: pane::Pane>(&mut self, pane: P) -> pane::PaneId<P> {
        self.panes.push(Box::new(pane));
        pane::PaneId {
            index: self.panes.len() - 1,
            _type: PhantomData,
        }
    }

    /// Generic access to any pane type stored in screen
    ///
    /// Reads the ID to match the box containing the pane and
    /// extracts a mut reference to it, casting to the typed stored
    /// in the [`PaneId`](crate::PaneId)
    pub fn pane<P: pane::Pane>(&mut self, id: pane::PaneId<P>) -> &mut P {
        /* Use slot ID to get Box of pane */
        let boxed: &mut Box<dyn pane::Pane> = &mut self.panes[id.index];

        /* Unbox inner for `trait Pane` (Still not struct type) */
        let some_pane: &mut dyn pane::Pane = boxed.as_mut();

        /* Widen to a `dyn Any` for downcast */
        let any: &mut dyn std::any::Any = some_pane;

        /* Checks type in box matches type 'P', then casts to type */
        any.downcast_mut::<P>().unwrap()
    }

    /// Renders all [`Screen::pane`] inside screen to the terminal
    /// In order of Pane added to screen
    pub fn draw(&mut self) -> io::Result<()> {
        for pane in &mut self.panes {
            let grid = pane.frame();
            let grid_string = grid.render_changes();

            /* Print grid to pane & return cursor back to original position */
            let mut out = io::stdout().lock();
            write!(out, "\x1b7{}\x1b8", grid_string)?;
            out.flush()?;
        }
        Ok(())
    }

    /// Switches to the alternate screen and clears it
    ///
    /// Also turns off autowrap, so writing the bottom-right cell cannot scroll
    /// the screen. Call [`Screen::teardown`] before exiting to give the user
    /// back their normal screen
    pub fn setup(&self) -> io::Result<()> {
        let mut out = io::stdout().lock();
        // alternate screen, clear, scroll region = rows 1..h-1
        write!(out, "\x1b[?1049h\x1b[2J")?;
        write!(out, "\x1b[?7l")?; // Autowrap off - prevent background screen scrolling
        //write!(out, "\x1b[{};{}H]\x1b[2K>", self.rows, 0)?;
        //write!(out, "\x1b[?1049h\x1b[2J\x1b[1;{}r", self.rows - 1)?;
        //write!(out, "\x1b[{};{}H]\x1b[2K>", self.rows, 0)?;
        out.flush()
    }

    /// Resets the scroll region and returns to the user's normal screen.
    pub fn teardown(&self) -> io::Result<()> {
        let mut out = io::stdout().lock();
        write!(out, "\x1b[r\x1b[?7h\x1b[?1049l")?;
        out.flush()
    }

    /**********************************************************
     *                       Private                          *
     **********************************************************/
    /// Reads the terminal size from `tput` into `rows` and `columns`
    ///
    /// # Errors
    /// Returns `NotFound` if the output of `tput` cannot be parsed
    ///
    /// # Panics
    /// Panics if `tput` cannot be run
    fn read_terminal_size(&mut self) -> io::Result<()> {
        let tput_cmd = Command::new("tput")
            .arg("lines")
            .arg("cols")
            .stdin(Stdio::inherit()) // Pass stdin to tput to read terminal
            .output()
            .expect("tput should return Lines & Cols of terminal");

        let screen_size = String::from_utf8_lossy(&tput_cmd.stdout).trim().to_string();

        let mut screen_dims = screen_size.split_whitespace().map(str::parse::<u32>);

        let (Some(Ok(lines)), Some(Ok(cols))) = (screen_dims.next(), screen_dims.next()) else {
            return Err(Error::new(
                ErrorKind::NotFound,
                "Unable to determan screen size from - {screen_dims}",
            ));
        };

        self.rows = lines as usize;
        self.columns = cols as usize;

        Ok(())
    }

    fn helper_print_screen_dims(&self) {
        println!("\n--- Main Screen ---");
        println!("Width:{} Height:{}", self.columns, self.rows);
    }
}
