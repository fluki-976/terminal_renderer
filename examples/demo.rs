//! Demo: a log that scrolls above a one-line command row.
//!
//! # Project overview
//!
//! **Aim:** an API-style rendering engine for the terminal, to support the
//! tty project.
//!
//! **Base features**
//! - Alternate screen for rendering (setup and teardown)
//! - Screen split into separate rectangles: logs on top, input on the bottom
//! - Get the screen size
//! - Update when the screen size changes (terminfo / tput)
//!
//! **Extended features**
//! - Command history
//! - Scroll logs (mouse wheel escape sequences, handled like PgUp / PgDn)
//! - Search

#[rustfmt::skip]
use std::{
    io,
    thread, 
    time, 
};

use terminal_renderer as tr;

fn main() -> io::Result<()> {
    /* Create and initialize new screen */
    let mut display = match tr::Screen::new() {
        Ok(disp) => disp,
        Err(e) => return Err(e),
    };
    display.setup()?;

    /* Create panes to create seperate sections on the screen to draw on */
    let grid_pane = display.add_pane(tr::Grid::new(0, display.rows - 1, display.columns, 1)?);
    let log_pane = display.add_pane(tr::LogPane::new(0, 0, display.columns, display.rows - 1)?);

    /* Styling panes with options */
    display.pane(log_pane).set_border(true)?;

    /* Writing to the panes */
    display.pane(grid_pane).fill('1');
    for i in 0..100 {
        display.pane(log_pane).push_line(&format!("Line: {}", i));
    }

    /* Draw everything buffered to the screen */
    display.draw()?;

    /* Simulating Rolling log for scroll tests */
    for i in 101..200 {
        display.pane(log_pane).push_line(&format!("Line: {}", i));
        thread::sleep(time::Duration::from_secs(1));
        display.draw()?;
    }

    /* Cleanup and hand back screen to user */
    display.teardown()?;
    Ok(())
}
