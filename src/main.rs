/*
 * Project Overview:
 *
 * Aim:
 *  API like Rendering engine for terminal to support tty project
 *
 * Base Features:
 *  Alternate screen for rendering (startup/teardown)
 *  Screen divide, inputs on bottom, logs on top (Seperate rectangles per block)
 *  Get screen size
 *  Update on screen size (terminfo/tput)
 *
 * Extended Features:
 *  History cmds
 *  Scroll logs (wheel specific /e cmds, handle like pg_up/dn)
 *  Search
 *
 */

#[rustfmt::skip]
use std::{
    collections::HashMap, 
    io::{self, BufWriter, Write, Error, ErrorKind}, 
    process::{Command, Stdio}, 
    thread, 
    time, 
    usize,
};

#[rustfmt::skip]
#[derive(Clone, Copy, Default)]
enum Color {
    #[default]
    Default,        // terminal's own colour (39 / 49)
    Indexed(u8),    // 0–15 basic and bright, 16–255 palette
    Rgb(u8, u8, u8),
}

#[derive(Clone, Copy, Default)]
struct Style {
    foreground: Color,
    background: Color,
    mods: u8, // BOLD = 1, ITALIC = 2, UNDERLINE = 4, REVERSE = 8, ...
}

/*
 * Singular character on screen
 * (Group same styles before setting escape sequence)
 */
#[derive(Default, Clone)]
struct Cell {
    style: Style,
    char: char,
}

/*****************************************************************************/
/*                                  Grid                                     */
/*****************************************************************************/
/*
 * Isolated section of screen for rendering in
 */
#[rustfmt::skip]
#[derive(Default)]
struct Grid {
    x: u32,            // X,Y Start position (0,0 - Top Left)
    y: u32,
    width: u32,        // Grid Dimentions
    height: u32,
    cells: Vec<Cell>,  // All cells in Grid
}

impl Grid {
    fn new(x: u32, y: u32, width: u32, height: u32) -> io::Result<Self> {
        let num_of_cells: usize = (width * height) as usize;
        let cells: Vec<Cell> = vec![Cell::default(); num_of_cells];
        Ok(Grid {
            x: x,
            y: y,
            width: width,
            height: height,
            cells: cells,
        })
    }

    fn write_cells(&mut self, line: String, mut line_offset: i32, char_offset: u32) {
        /*
         * Place line into grid awaiting rendering
         *
         * param offset: + Offset from top
         *               -1 New last line (Common with scrol lock)
         */


        if line_offset < -1 || line_offset > self.height as i32 {
            println!("Line offset invalid: {} (-2 < x < {})", line_offset, self.height);
            return;
        } else if char_offset > self.width {
            /* Can be negative beyone -1 */
            println!("Char offset invalid: {} (x < {})", char_offset, self.width);
            return;
        } else if line_offset == -1 {
            line_offset = (self.height - 1) as i32;
        }

        let cell_offset: usize = (line_offset as usize) * (self.width as usize) + (char_offset as usize);

        for (i, c) in line.chars().enumerate() {
            if cell_offset + i >= (self.width as usize) {
                break; // TODO: Implement line wrapping
            }
            self.cells[cell_offset + i].char = c;
            println!("i {} at {}", c, cell_offset+i);
        }

        for i in &self.cells[(cell_offset-char_offset as usize)..cell_offset+line.len()] {
            if i.char == char::default() {
                print!(".");
            } else {
                print!("{}",i.char);
            }
        }
        print!("\n");
    }

    fn helper_print_grid_dims(&self) {
        println!("Start: {}:{}",self.x,self.y);
        println!("Rows:{} Cols:{}",self.height,self.width);
        println!("Total Cels: {}",self.cells.len());
    }
}

/*****************************************************************************/
/*                                 Screen                                    */
/*****************************************************************************/
#[derive(Default)]
struct Screen {
    rows: u32,
    columns: u32,
    grids: HashMap<String, Grid>,
}

impl Screen {
    fn new() -> io::Result<Self> {
        let mut screen = Self::default();
        screen.get_screen_size()?;
        Ok(screen)
    }

    fn get_screen_size(&mut self) -> io::Result<()> {
        /*
         * Get/Set the current dimentions on the terminal
         */
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

        self.rows = lines;
        self.columns = cols;

        Ok(())
    }

    fn add_grid(&mut self, grid_name: String, new_grid: Grid) -> io::Result<()> {
        if self.grids.contains_key(&grid_name) {
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                "Grid name '{grid_name}' already exists",
            ));
        }
        self.grids.insert(grid_name, new_grid);
        Ok(())
    }

    fn draw_grid(&mut self, grid_name: String) {

        

    }



    fn setup_screen(&self) -> io::Result<()> {
        /*
         * Setup New alternate screen for rendering
         *
         * Scroll lock on all but lowest row
         * - Log in top section
         * - Cmds on lowest line
         */
        let mut out = io::stdout().lock();
        // alternate screen, clear, scroll region = rows 1..h-1
        write!(out, "\x1b[?1049h\x1b[2J")?;
        //write!(out, "\x1b[{};{}H]\x1b[2K>", self.rows, 0)?;
        //write!(out, "\x1b[?1049h\x1b[2J\x1b[1;{}r", self.rows - 1)?;
        //write!(out, "\x1b[{};{}H]\x1b[2K>", self.rows, 0)?;
        out.flush()
    }

    fn teardown(&self) -> io::Result<()> {
        /*
         * Revert back to terminal screen
         */
        let mut out = io::stdout().lock();
        write!(out, "\x1b[r\x1b[?1049l")?;
        out.flush()
    }






    fn helper_print_screen_dims(&self) {
        println!("\n--- Main Screen ---");
        println!("Width:{} Height:{}",self.columns, self.rows);
    }

    fn helper_list_grids(&self) {
        for k in self.grids.keys() {
            println!("\n--- Grid: {} ---", k);
            self.grids[k].helper_print_grid_dims();
        }
    }
}

/*

fn clear_screen() {
    println!("\x1b[2J");
}



fn restore_screen() -> io::Result<()> {
    let mut out = io::stdout().lock();
    // reset scroll region, leave alternate screen
    write!(out, "\x1b[r\x1b[?1049l")?;
    out.flush()
}
*/

fn main() -> io::Result<()> {
    let mut display = match Screen::new() {
        Ok(disp) => disp,
        Err(e) => return Err(e),
    };
    display.setup_screen()?;

    // New Grid using all but bottom Row
    let log_grid = Grid::new(0, 0, display.columns, display.rows-1)?;
    display.add_grid("log".to_string(), log_grid)?;

    // New Grid using only bottom row
    let cmd_grid = Grid::new(0, display.rows-1, display.columns, 1)?;
    display.add_grid("cmd".to_string(), cmd_grid)?;


    display.helper_print_screen_dims();
    display.helper_list_grids();


    display.grids
        .get_mut("cmd")
        .expect("cmd Grid Should be Set")
        .write_cells("Hello_COMMD\n".to_string(), 0, 158);


    display.grids
        .get_mut("log")
        .expect("cmd Grid Should be Set")
        .write_cells("!!!lo_CO123\n".to_string(), 0, 159);

    display.grids
        .get_mut("log")
        .expect("cmd Grid Should be Set")
        .write_cells("1!!lo_COMMD\n".to_string(), 0, 160);


/*    display.grids
        .get_mut("log")
        .expect("cmd Grid Should be Set")
        .write_cells("Hello_LOGGR\n".to_string(), 5, 20);


    thread::sleep(time::Duration::from_secs(1));


    let mut x = 0;
    loop {
        display.grids
            .get_mut("log")
            .expect("cmd Grid Should be Set")
            .write_cells("new log\n".to_string(), -1, 20);

        thread::sleep(time::Duration::from_secs(1));

        if x == 10 {
            break;
        }
        x+=1;
    }
*/


    thread::sleep(time::Duration::from_secs(3));
    display.teardown()?;

    Ok(())
}
