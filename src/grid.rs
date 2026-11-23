use std::io::{Error, ErrorKind};
#[rustfmt::skip]
use std::{
    io::{self}, 
    usize,
};

use super::Pane;
use crate::cell;

/// Characters that make up a border
#[derive(Clone, Copy)]
struct BorderChars {
    top_left: char,
    top_right: char,
    bottom_left: char,
    bottom_right: char,
    horizontal: char,
    vertical: char,
}

impl BorderChars {
    /// Single line box drawing
    const SINGLE: Self = Self {
        top_left: '┌',      //'\u{250C}'
        top_right: '┐',     //'\u{2510}'
        bottom_left: '└',   //'\u{2514}'
        bottom_right: '┘',  //'\u{2518}'
        horizontal: '─',    //'\u{2500}'
        vertical: '│',      //'\u{2502}'
    };

    /// Blank cells, matching `Cell::default`, used to remove a border
    const BLANK: Self = Self {
        top_left: ' ',
        top_right: ' ',
        bottom_left: ' ',
        bottom_right: ' ',
        horizontal: ' ',
        vertical: ' ',
    };
}

impl Default for BorderChars {
    fn default() -> Self {
        Self::SINGLE
    }
}

#[derive(Default)]
struct Border {
    enable: bool,
    style: cell::Style,
    /// Drawn when the border is on
    chars: BorderChars,
}

/*****************************************************************************/
/*                                  Grid                                     */
/*****************************************************************************/
/// A rectangle of cells at a fixed position on screen.
///
/// Keeps this frame's cells and the last drawn frame's, so only the cells
/// that changed are redrawn.
#[rustfmt::skip]
#[derive(Default)]
pub struct Grid {
    /// Column of the left edge, 0-based (0 = left of the terminal).
    x: usize,
    /// Row of the top edge, 0-based (0 = top of the terminal).
    y: usize,
    /// Width in cells.
    width: usize,
    /// Height in cells.
    height: usize,
    /// This frame, row by row: cell (row, col) is at `row * width + col`.
    cells: Vec<cell::Cell>,
    /// The frame last drawn, compared against `cells` to find changes.
    prev_cells: Vec<cell::Cell>,
    /// Show or hide boarder
    border: Border,
}

impl Pane for Grid {
    /// A grid is already a frame, so it returns itself unchanged.
    fn frame(&mut self) -> &mut Grid {
        self
    }
}

impl Grid {
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> io::Result<Self> {
        let num_of_cells: usize = width * height;
        let cells: Vec<cell::Cell> = vec![cell::Cell::default(); num_of_cells];
        Ok(Grid {
            x: x,
            y: y,
            width: width,
            height: height,
            cells: cells.clone(),
            prev_cells: cells.clone(),
            border: Border::default(),
        })
    }

    /**********************************************************
     *                       Public                           *
     **********************************************************/
    pub fn clear(&mut self) {
        self.cells.fill(cell::Cell::default());
        if self.border.enable {
            self.draw_border(self.border.chars);
        }
    }

    pub fn fill_grid(&mut self, input: char) {
        for cell in &mut self.cells {
            cell.char = input;
        }
    }

    /// Turns the one-cell border around the grid on or off
    ///
    /// The border takes the outer ring of cells, leaving the inside for
    /// content
    ///
    /// Does nothing if the border is already in the requested state
    ///
    /// # Errors
    /// `InvalidInput` if the grid is smaller than 3x3, leaving no room inside
    pub fn set_border(&mut self, enable: bool) -> io::Result<()> {
        /* Nothing to draw or clear */
        if self.border.enable == enable {
            return Ok(());
        }

        /* A border needs at least one cell inside it */
        if enable && (self.width < 3 || self.height < 3) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Grid must be atlest 3x3 to have a border",
            ));
        }

        self.border.enable = enable;
        if enable {
            self.draw_border(self.border.chars);
        } else {
            self.draw_border(BorderChars::BLANK);
        }
        Ok(())
    }

    /**********************************************************
     *                        Crate                           *
     **********************************************************/
    /// Copies `line` into the inside of the grid, starting at the given row
    /// and column
    ///
    /// Positions are inside the border when one is set, so row 0, column 0
    /// is the first cell within it. Cells past the inner right edge are cut
    /// off, not wrapped. An offset outside the inside prints an error to
    /// stdout and writes nothing
    ///
    /// # Arguments
    /// * `line` - Cells to copy, left to right
    /// * `line_offset` - Row to write, 0-based from the top of the inside
    /// * `char_offset` - Column of the first cell, 0-based from the left of
    ///   the inside
    pub(crate) fn write_cells(
        &mut self,
        line: &[cell::Cell],
        line_offset: usize,
        char_offset: usize,
    ) {
        /* Use adjusted width & height to account for optional border */
        let (height, width) = self.get_grid_size();
        if line_offset >= height || char_offset >= width {
            println!(
                "Offset invalid: row {line_offset}, col {char_offset} (inside is {height}x{width})"
            );
            return;
        }

        /* Inner row and column to an index in the outer cells */
        let inset = self.inset();
        let start = (line_offset + inset) * self.width + char_offset + inset;

        /* Cut off at the inner right edge. TODO: line wrapping */
        let len = line.len().min(width - char_offset);
        self.cells[start..start + len].copy_from_slice(&line[..len]);
    }

    /// Returns the escape string that redraws only the cells changed since
    /// the last call, then records this frame as drawn.
    ///
    /// Each run of adjacent changed cells, and each new row, starts with one
    /// cursor move. Returns an empty string when nothing changed. Cell styles
    /// are not emitted yet.
    pub(crate) fn build_output_string(&mut self) -> String {
        let mut grid_string: String = String::default();

        /* Identify evey cell in grid changed from last write */
        let changed: Vec<usize> = self
            .cells
            .iter()
            .zip(&self.prev_cells)
            .enumerate()
            .filter(|(_, (cur, prev))| cur != prev)
            .map(|(i, _)| i)
            .collect();

        if changed.len() == 0 {
            return "".to_string();
        }

        /* Move the cursor on a gap in changed cells, or at the start of a grid row */
        let mut next_cell_id: Option<usize> = None;
        for cell_id in changed {
            if next_cell_id != Some(cell_id) || cell_id % self.width == 0 {
                // TODO: verify best crossover point from move to
                // replace current chars with same
                /* Move Cursor to cell Location */
                let row = (cell_id / self.width) + self.y + 1;
                let col = (cell_id % self.width) + self.x + 1;
                grid_string.push_str(&format!("\x1b[{};{}H", row, col));
            }
            /* Appending chars to single string */
            grid_string.push_str(&format!("{}", self.cells[cell_id].char));
            next_cell_id = Some(cell_id + 1);
        }

        /* Store current cells for next frame comparison */
        self.prev_cells = self.cells.clone();

        grid_string
    }

    pub(crate) fn get_grid_size(&self) -> (usize, usize) {
        (
            self.height - 2 * self.inset(),
            self.width - 2 * self.inset(),
        )
    }

    /**********************************************************
     *                       Private                          *
     **********************************************************/
    /// Accounts for border width
    fn inset(&self) -> usize {
        usize::from(self.border.enable)
    }

    /// Writes the border chars into the outer ring of cells, or blanks the
    /// ring when the border is off
    fn draw_border(&mut self, chars: BorderChars) {
        let (width, height) = (self.width, self.height);
        /* Closure for direct x,y cell access */
        let mut set = |row: usize, col: usize, ch: char| self.cells[row * width + col].char = ch;

        /* Top and bottom edges */
        for col in 1..width - 1 {
            set(0, col, chars.horizontal);
            set(height - 1, col, chars.horizontal);
        }
        /* Left and right edges */
        for row in 1..height - 1 {
            set(row, 0, chars.vertical);
            set(row, width - 1, chars.vertical);
        }
        /* Corners */
        set(0, 0, chars.top_left);
        set(0, width - 1, chars.top_right);
        set(height - 1, 0, chars.bottom_left);
        set(height - 1, width - 1, chars.bottom_right);
    }

    /**********************************************************
     *                       Helper                           *
     **********************************************************/
    pub fn helper_print_grid_dims(&self) {
        println!("Start: {}:{}", self.x, self.y);
        println!("Rows:{} Cols:{}", self.height, self.width);
        println!("Total Cels: {}", self.cells.len());
    }

    /// Text of one row, empty cells left out
    ///
    /// Give tests visiblity of the expected output to
    /// the screen
    #[cfg(test)]
    pub(crate) fn row_text(&self, row: usize) -> String {
        let text: String = self.cells[row * self.width..(row + 1) * self.width]
            .iter()
            .map(|cell| cell.char)
            .collect();
        text.trim_end_matches(' ').to_string()
    }
    /// Text of every row top to bottom, border rows included
    #[cfg(test)]
    pub(crate) fn rows_text(&self) -> Vec<String> {
        (0..self.height).map(|row| self.row_text(row)).collect()
    }
    #[cfg(test)]
    pub(crate) fn border_enabled(&self) -> bool {
        self.border.enable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod set_border {
        use super::*; 

        #[test]
        fn no_border_on_small_grid() {
            let mut grid = Grid::new(0, 0, 10, 2).unwrap();
            let result = grid.set_border(true);
            assert!(result.is_err(), "{result:?}");
        }

        #[test]
        fn only_enable_boarder_once() {
            let mut grid = Grid::new(0, 0, 10, 6).unwrap();
            let result_a = grid.set_border(true);
            let result_b = grid.set_border(true);

            /* Second enable does nothing */
            assert!(result_a.is_ok(), "{result_a:?}");
            assert!(result_b.is_ok(), "{result_b:?}");
            assert!(grid.border_enabled());
        }
    }

    mod draw_border {
        use super::*; 

        fn grid_with_border(x: usize, y: usize, width: usize, height: usize) -> Grid {
            let mut grid = Grid::new(x, y, width, height).unwrap();
            let result = grid.set_border(true);
            assert!(result.is_ok(), "{result:?}");
            grid
        }

        #[test]
        fn border_drawn_in_outer_grid_cells() {
            let mut grid = grid_with_border(0, 0, 10, 6);
            grid.draw_border(grid.border.chars);
            let rows: Vec<String> = (0..6).map(|i| grid.row_text(i)).collect();
            assert_eq!(rows[0], "┌────────┐");
            assert_eq!(rows[1], "│        │");
            assert_eq!(rows[2], "│        │");
            assert_eq!(rows[3], "│        │");
            assert_eq!(rows[4], "│        │");
            assert_eq!(rows[5], "└────────┘");
        }

        #[test]
        fn border_cleared_when_unset() {
            let mut grid = grid_with_border(0, 0, 10, 6);
            let result = grid.set_border(false);
            assert!(result.is_ok(), "{result:?}");

            let rows: Vec<String> = (0..6).map(|i| grid.row_text(i)).collect();
            assert_eq!(rows, ["", "", "", "", "", ""]);
        }

        #[test]
        fn first_row_with_border_hits_inside_border() {
            let mut grid = grid_with_border(0, 0, 10, 6);
            grid.write_cells(&cell::cells_from("Hello"), 0, 0);
            grid.draw_border(grid.border.chars);
            let rows: Vec<String> = (0..6).map(|i| grid.row_text(i)).collect();
            assert_eq!(rows[0], "┌────────┐");
            assert_eq!(rows[1], "│Hello   │");
            assert_eq!(rows[2], "│        │");
            assert_eq!(rows[3], "│        │");
            assert_eq!(rows[4], "│        │");
            assert_eq!(rows[5], "└────────┘");
        }

        #[test]
        fn border_remains_after_clear() {
            let mut grid = grid_with_border(0, 0, 10, 6);
            grid.draw_border(grid.border.chars);
            grid.clear();
            let rows: Vec<String> = (0..6).map(|i| grid.row_text(i)).collect();
            assert_eq!(rows[0], "┌────────┐");
            assert_eq!(rows[1], "│        │");
            assert_eq!(rows[2], "│        │");
            assert_eq!(rows[3], "│        │");
            assert_eq!(rows[4], "│        │");
            assert_eq!(rows[5], "└────────┘");
        }
    }
}
