#[rustfmt::skip]
use std::{
    io::{self}, 
    usize,
};

use crate::cell;
use super::Pane;


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
        })
    }

    /**********************************************************
     *                       Public                           *
     **********************************************************/
    pub fn clear(&mut self) {
        self.cells.fill(cell::Cell::default());
    }

    pub fn fill_grid(&mut self, input: char) {
        for cell in &mut self.cells {
            cell.char = input;
        }
    }

    /**********************************************************
     *                        Crate                           *
     **********************************************************/
    /// Copies `line` into the grid, starting at the given row and column.
    ///
    /// Cells past the right edge are cut off, not wrapped. An offset outside
    /// the grid prints an error to stdout and writes nothing.
    ///
    /// # Arguments
    /// * `line` - Cells to copy, left to right.
    /// * `line_offset` - Row to write, 0-based from the top of the grid.
    /// * `char_offset` - Column of the first cell, 0-based from the left of
    ///   the grid.
    pub(crate) fn write_cells(&mut self, line: &Vec<cell::Cell>, line_offset: usize, char_offset: usize) {
        if char_offset >= self.width {
            println!("Char offset invalid: {} (col < {})", char_offset, self.width);
            return;
        } else if line_offset >= self.height {
            println!("Line offset invalid: {} (row < {}", line_offset, self.height);
            return;
        }

        let cell_offset: usize =
            (line_offset as usize) * self.width + char_offset;

        // TODO Could be converted to a memcpy like transfer
        for (i, ch) in line.iter().enumerate() {
            /* Loc in current line + length >= grid width  */
            if (cell_offset % self.width as usize) + i >= (self.width as usize) {
                break; // TODO: Implement line wrapping
            }
            self.cells[cell_offset + i] = ch.clone();

        }
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
        let changed: Vec<usize> = self.cells
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
        (self.height, self.width)
    }

    /**********************************************************
     *                       Private                          *
     **********************************************************/

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
        let text: String = self.cells[ row * self.width .. (row + 1) * self.width]
            .iter()
            .map(|cell| cell.char)
            .collect();
        text.trim_end_matches('\0').to_string()
    }
}





#[cfg(test)]
mod tests {
    use super::*;



}
