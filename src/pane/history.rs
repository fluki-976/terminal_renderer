use std::collections::VecDeque;
use std::io;

use super::{Grid, Pane};
use crate::{Style, cell};

/*****************************************************************************/
/*                                LogPane                                    */
/*****************************************************************************/
pub struct LogPane {
    /// Every line received so far, up to the log limit, and the scroll
    /// position
    log: LogHistory,
    /// Frame that the visible part of `log` is copied into before each
    /// draw
    grid: Grid,
}

impl LogPane {
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> io::Result<Self> {
        Ok(LogPane {
            grid: Grid::new(x, y, width, height)?,
            log: LogHistory::new(height),
        })
    }

    pub fn push_line(&mut self, line: &str) {
        self.log.push_line(line);
    }

    /// Turns the border around the pane on or off
    ///
    /// The scroll limit follows the rows left inside the border
    pub fn set_border(&mut self, enable: bool) -> io::Result<()> {
        self.grid.set_border(enable)?;
        self.log.set_view_height(self.grid.get_grid_size().0);
        Ok(())
    }

    /// Moves the view `scroll_n_lines` towards older lines
    ///
    /// Stops once the oldest line is on the top row. While scrolled, the
    /// view stays on the same lines as new ones are pushed
    pub fn scroll_up(&mut self, scroll_n_lines: usize) {
        self.log
            .set_scroll_offset(self.log.scroll.saturating_add(scroll_n_lines));
    }

    /// Moves the view `scroll_n_lines` towards newer lines
    ///
    /// Stops at the newest line, where the view follows new lines again
    pub fn scroll_down(&mut self, scroll_n_lines: usize) {
        self.log
            .set_scroll_offset(self.log.scroll.saturating_sub(scroll_n_lines));
    }
}

impl Pane for LogPane {
    /// Copies the visible lines from log into the grid and returns it
    ///
    /// Shows the `height` lines that end `scroll` lines back from the newest.
    /// Lines wider than the grid are cut off, not wrapped
    fn frame(&mut self) -> &mut Grid {
        let (height, _) = self.grid.get_grid_size();

        self.grid.clear();

        /* Takes from the latest lines, starting from the back */
        let visible = self
            .log
            .lines
            .iter()
            .rev()
            .skip(self.log.scroll)
            .take(height);
        for (row, line) in visible.enumerate() {
            self.grid.write_cells(line, height - 1 - row, 0);
        }
        &mut self.grid
    }
}

/*****************************************************************************/
/*                               LogHistory                                  */
/*****************************************************************************/
pub(crate) struct LogHistory {
    /// Received lines, oldest first, stored unwrapped
    lines: VecDeque<Vec<cell::Cell>>,
    /// How many lines are shown, used to clamp scroll to full grids
    view_height: usize,
    /// Most lines kept. The oldest line is dropped once this is exceeded
    max_lines: usize,
    /// Most characters stored in one line. Guards against input that never
    /// sends a newline
    max_line_len: usize,
    /// How many lines back from the newest the view ends. 0 follows new
    /// output
    scroll: usize,
}

impl LogHistory {
    /**********************************************************
     *                       Public                           *
     **********************************************************/
    pub fn new(height: usize) -> Self {
        LogHistory {
            lines: VecDeque::new(),
            view_height: height,
            max_lines: 1000,
            max_line_len: 1500,
            scroll: 0,
        }
    }

    /// Adds `line` to the log as a new line, in the default style
    ///
    /// Every `\n` starts a further line, including a trailing one, so
    /// `"one\n"` stores `"one"` and an empty line. Strip it first if that
    /// is not wanted. `\r` is ignored. Once the current line holds
    /// `max_line_len` characters, the rest of that line is dropped
    pub(crate) fn push_line(&mut self, line: &str) {
        /* Each push starts its own line */
        self.new_line();

        for ch in line.chars() {
            match ch {
                '\n' => self.new_line(),
                '\r' => {}
                c => {
                    let current_line = self.lines.back_mut().unwrap();
                    if current_line.len() >= self.max_line_len {
                        continue;
                        // TODO: Error handle overflow line characters
                    }
                    current_line.push(cell::Cell {
                        char: c,
                        style: Style::default(),
                    });
                }
            }
        }
    }

    /**********************************************************
     *                       Private                          *
     **********************************************************/
    /// Adds new line to write to
    ///
    /// Does not move rendered view by adjusting the scroll
    /// if already non zero
    fn new_line(&mut self) {
        self.lines.push_back(Vec::new());
        self.trim_line_history();

        /* Don`t shift text on screen if at scroll offset */
        if self.scroll > 0 {
            self.set_scroll_offset(self.scroll + 1);
        }
    }

    /// Drops the oldest line once log holds more than `max_lines`
    fn trim_line_history(&mut self) {
        if self.lines.len() > self.max_lines {
            /* Scroll counts from the newest line, so this leaves it unchanged */
            self.lines.pop_front();
        }
    }

    /// Sets how many lines are shown, pulling scroll back inside the new
    /// limit when the view grows
    fn set_view_height(&mut self, view_height: usize) {
        self.view_height = view_height;
        self.set_scroll_offset(self.scroll);
    }

    /// Sets how many lines back from the newest the view ends, limited to
    /// [`max_scroll`](Self::max_scroll)
    fn set_scroll_offset(&mut self, scroll_offset: usize) {
        self.scroll = scroll_offset.min(self.max_scroll());
    }

    /// Furthest scroll that still fills the view: `view_height` lines short
    /// of the line count
    fn max_scroll(&self) -> usize {
        self.lines.len().saturating_sub(self.view_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod log_pane {
        use super::*;

        /// 10-wide pane `height` rows high, holding `lines` oldest first
        fn pane_with(height: usize, lines: &[&str]) -> LogPane {
            let mut pane = LogPane::new(0, 0, 10, height).unwrap();
            for line in lines {
                pane.push_line(line);
            }
            pane
        }

        /// Text of each grid row, top to bottom, border rows included
        fn get_rows_from_grid(grid: &Grid) -> Vec<String> {
            grid.rows_text()
        }

        #[test]
        fn empty_history_draws_blank() {
            let mut log_pane = pane_with(5, &[]);

            let rows = get_rows_from_grid(log_pane.frame());

            assert_eq!(rows, ["", "", "", "", ""]);
        }

        #[test]
        fn short_history_sits_at_bottom() {
            let mut log_pane = pane_with(5, &["First", "Second"]);

            let rows = get_rows_from_grid(log_pane.frame());

            assert_eq!(rows, ["", "", "", "First", "Second"]);
        }

        #[test]
        fn full_pane_shows_newest_lines() {
            let mut log_pane =
                pane_with(4, &["First", "Second", "Third", "Fourth", "Fifth", "Sixth"]);

            let rows = get_rows_from_grid(log_pane.frame());

            assert_eq!(rows, ["Third", "Fourth", "Fifth", "Sixth"]);
        }

        mod scroll {
            use super::*;

            /// More lines than the 4-row test pane, so there is room to scroll
            const SEVEN_LINES: &[&str] = &[
                "First", "Second", "Third", "Fourth", "Fifth", "Sixth", "Seventh",
            ];

            #[test]
            fn scroll_up_shows_older_lines() {
                let mut log_pane = pane_with(4, SEVEN_LINES);
                log_pane.scroll_up(2);

                let rows = get_rows_from_grid(log_pane.frame());

                assert_eq!(rows, ["Second", "Third", "Fourth", "Fifth"]);
            }

            #[test]
            fn scroll_up_stops_with_oldest_line_at_top() {
                let mut log_pane = pane_with(4, SEVEN_LINES);
                log_pane.scroll_up(10);

                let rows = get_rows_from_grid(log_pane.frame());

                assert_eq!(rows, ["First", "Second", "Third", "Fourth"]);
            }

            #[test]
            fn scroll_up_does_nothing_when_lines_fit_pane() {
                let mut log_pane = pane_with(4, &["First", "Second"]);
                log_pane.scroll_up(1);

                let rows = get_rows_from_grid(log_pane.frame());

                assert_eq!(rows, ["", "", "First", "Second"]);
            }

            #[test]
            fn scroll_down_moves_back_toward_newest() {
                let mut log_pane = pane_with(4, SEVEN_LINES);
                log_pane.scroll_up(3);
                log_pane.scroll_down(1);

                let rows = get_rows_from_grid(log_pane.frame());

                assert_eq!(rows, ["Second", "Third", "Fourth", "Fifth"]);
            }

            #[test]
            fn scroll_down_stops_at_newest() {
                let mut log_pane = pane_with(4, SEVEN_LINES);
                log_pane.scroll_up(1);
                log_pane.scroll_down(5);

                let rows = get_rows_from_grid(log_pane.frame());

                assert_eq!(rows, ["Fourth", "Fifth", "Sixth", "Seventh"]);
            }

            #[test]
            fn border_off_while_scrolled_fills_pane() {
                let mut log_pane = pane_with(6, SEVEN_LINES);
                log_pane.set_border(true).unwrap();
                log_pane.scroll_up(10);
                log_pane.set_border(false).unwrap();

                let rows = get_rows_from_grid(log_pane.frame());

                /* Scroll pulled back so all 6 rows hold lines, none blank */
                assert_eq!(
                    rows,
                    ["First", "Second", "Third", "Fourth", "Fifth", "Sixth"]
                );
            }

            #[test]
            fn scroll_up_stops_with_oldest_line_at_top_of_border() {
                let mut log_pane = pane_with(6, SEVEN_LINES);
                log_pane.set_border(true).unwrap();
                log_pane.scroll_up(10);

                let rows = get_rows_from_grid(log_pane.frame());

                /* 4 rows inside the border, filled by the 4 oldest lines */
                assert_eq!(
                    rows,
                    [
                        "┌────────┐",
                        "│First   │",
                        "│Second  │",
                        "│Third   │",
                        "│Fourth  │",
                        "└────────┘"
                    ]
                );
            }
        }
    }

    mod log_history {
        use super::*;

        /// View height for tests that use LogHistory without a pane. Scroll is
        /// limited only by the line count, as if the view were 0 rows tall
        const NO_VIEW_LIMIT: usize = 0;

        /// Text of each stored line, so asserts compare plain strings
        fn texts(log: &LogHistory) -> Vec<String> {
            log.lines
                .iter()
                .map(|line| line.iter().map(|cell| cell.char).collect())
                .collect()
        }

        /// Log holding the lines "0", "1", ... up to `n - 1`
        fn log_with_lines(n: usize) -> LogHistory {
            let mut log = LogHistory::new(NO_VIEW_LIMIT);
            for i in 0..n {
                log.push_line(&i.to_string());
            }
            log
        }

        /// Log holding lines "0" to "num_of_lines - 1", shown in a view
        /// `height` rows high
        fn log_in_view(height: usize, num_of_lines: usize) -> LogHistory {
            let mut log = LogHistory::new(height);
            for i in 0..num_of_lines {
                log.push_line(&i.to_string());
            }
            log
        }

        #[test]
        fn new_log_is_empty_and_unscrolled() {
            let log = LogHistory::new(5);
            assert!(log.lines.is_empty());
            assert_eq!(log.scroll, 0);
        }

        mod push_line {
            use super::*;

            #[test]
            fn push_empty_line_to_history() {
                let mut log = LogHistory::new(NO_VIEW_LIMIT);
                log.push_line("");
                assert_eq!(texts(&log), [""]);
            }

            #[test]
            fn each_push_is_its_own_line() {
                let mut log = LogHistory::new(NO_VIEW_LIMIT);
                log.push_line("one");
                log.push_line("two");
                assert_eq!(texts(&log), ["one", "two"]);
            }

            #[test]
            fn new_line_code_puts_text_on_next_line() {
                let mut log = LogHistory::new(NO_VIEW_LIMIT);
                log.push_line("one\ntwo");
                assert_eq!(texts(&log), ["one", "two"]);
            }

            #[test]
            fn trailing_newline_adds_empty_line() {
                let mut log = LogHistory::new(NO_VIEW_LIMIT);
                log.push_line("one\n");
                assert_eq!(texts(&log), ["one", ""]);
            }

            #[test]
            fn overlong_line_cut_at_max_line_length() {
                let mut log = LogHistory::new(NO_VIEW_LIMIT);

                // S - Start, x - Filler, E - End, for identifying which end trimmed
                let long_line = format!("S{}E", "x".repeat(log.max_line_len + 100));
                log.push_line(&long_line);

                let log_line = texts(&log).last().unwrap().clone();
                assert_eq!(log_line.len(), log.max_line_len);
                assert!(log_line.starts_with('S'));
            }

            #[test]
            fn crlf_is_one_line_break() {
                let mut log = LogHistory::new(NO_VIEW_LIMIT);
                log.push_line("one\r\ntwo");
                assert_eq!(texts(&log), ["one", "two"]);
            }

            #[test]
            fn text_after_newline_survives_overlong_line() {
                let mut log = LogHistory::new(NO_VIEW_LIMIT);
                let long = "x".repeat(log.max_line_len + 10);
                log.push_line(&format!("{long}\nnext"));
                assert_eq!(texts(&log).last().unwrap(), "next");
            }
        }

        mod new_line {
            use super::*;

            #[test]
            fn adds_empty_line_at_newest_end() {
                let mut log = log_with_lines(3);
                log.new_line();
                assert_eq!(texts(&log), ["0", "1", "2", ""]);
            }

            #[test]
            fn unscrolled_view_keeps_following_newest() {
                let mut log = log_with_lines(3);
                log.new_line();
                assert_eq!(log.scroll, 0);
            }

            #[test]
            fn scrolled_view_holds_its_place() {
                let mut log = log_with_lines(10);
                log.set_scroll_offset(3);
                log.new_line();
                assert_eq!(log.scroll, 4);
            }

            #[test]
            fn scrolled_view_holds_its_place_when_history_full() {
                let mut log = log_with_lines(LogHistory::new(NO_VIEW_LIMIT).max_lines);
                log.set_scroll_offset(3);
                log.new_line();
                assert_eq!(log.scroll, 4);
            }

            #[test]
            fn scroll_stops_at_top_when_tracked_line_dropped() {
                let mut log = log_with_lines(LogHistory::new(NO_VIEW_LIMIT).max_lines);
                log.set_scroll_offset(log.max_lines);
                log.new_line();
                assert_eq!(log.scroll, log.max_lines);
            }
        }

        mod trim_line_history {
            use super::*;

            #[test]
            fn remove_oldest_line_on_overflow() {
                let mut log = log_with_lines(LogHistory::new(NO_VIEW_LIMIT).max_lines);
                /* One over max, bypassing push_line so only the trim runs */
                log.lines.push_back(cell::cells_from("NEW"));

                log.trim_line_history();

                /* Line "0" dropped to get back to max_lines */
                assert_eq!(log.lines.len(), log.max_lines);
                assert_eq!(texts(&log).first().unwrap(), "1");
                assert_eq!(texts(&log).last().unwrap(), "NEW");
            }

            #[test]
            fn scroll_unchanged_when_oldest_dropped() {
                let mut log = log_with_lines(LogHistory::new(NO_VIEW_LIMIT).max_lines);
                log.set_scroll_offset(3);
                log.lines.push_back(cell::cells_from("NEW"));

                log.trim_line_history();

                assert_eq!(log.scroll, 3);
            }

            #[test]
            fn adding_line_with_room_does_not_remove_lines() {
                let mut log = LogHistory::new(NO_VIEW_LIMIT);
                log.push_line("Old Line");
                log.push_line("New Line");

                log.trim_line_history();

                assert_eq!(texts(&log).first().unwrap(), "Old Line");
                assert_eq!(texts(&log).last().unwrap(), "New Line");
            }
        }

        mod set_view_height {
            use super::*;

            #[test]
            fn taller_view_pulls_scroll_back_inside_limit() {
                let mut log = log_in_view(4, 10);
                log.set_scroll_offset(6);

                log.set_view_height(8);

                /* 10 lines with 8 on screen leaves only 2 to scroll back through */
                assert_eq!(log.scroll, 2);
            }
        }

        mod set_scroll_offset {
            use super::*;

            #[test]
            fn setting_scroll_offset() {
                let mut log = log_with_lines(50);
                log.set_scroll_offset(5);
                assert_eq!(log.scroll, 5);
            }

            #[test]
            fn stop_scroll_past_lines() {
                let num_of_lines: usize = 50;
                let mut log = log_with_lines(num_of_lines);
                log.set_scroll_offset(num_of_lines + 1);
                assert_eq!(log.scroll, num_of_lines);
            }

            #[test]
            fn stop_scroll_past_max_lines() {
                let mut log = log_with_lines(LogHistory::new(NO_VIEW_LIMIT).max_lines);
                log.set_scroll_offset(log.max_lines + 5);
                assert_eq!(log.scroll, log.max_lines);
            }

            #[test]
            fn stop_scroll_on_empty_log() {
                let mut log = LogHistory::new(NO_VIEW_LIMIT);
                log.set_scroll_offset(1);
                assert_eq!(log.scroll, 0);
            }
        }

        mod max_scroll {
            use super::*;

            #[test]
            fn leaves_view_height_of_lines_on_screen() {
                let log = log_in_view(4, 10);

                /* 10 lines with 4 on screen leaves 6 to scroll back through */
                assert_eq!(log.max_scroll(), 6);
            }

            #[test]
            fn zero_scroll_when_lines_fit_view() {
                let log = log_in_view(4, 2);

                /* 2 lines fit on 4 height screen so nothing to scroll through */
                assert_eq!(log.max_scroll(), 0);
            }
        }
    }
}
