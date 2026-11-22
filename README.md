# terminal_renderer

A small terminal rendering library for Rust, written with no dependencies.
It splits the screen into panes, keeps a scrollable log, and redraws only
the cells that changed. It is built to be the display layer of the tty
project, and as a way to learn Rust and test-driven development

## What it can do

- **Alternate screen:** `Screen::setup_screen` switches to the alternate
  screen and turns off autowrap. `Screen::teardown` gives the user back their
  normal screen
- **Terminal size:** read from `tput` when the `Screen` is created
- **Panes:** any type that implements `Pane` can be added to a `Screen`.
  Panes are drawn in the order they were added
- **Typed pane handles:** `add_pane` returns a `PaneId<P>`. Passing it to
  `Screen::pane` gives back the pane as its own type, so pane-specific
  methods such as `push_line` work without casting
- **Diff rendering:** each `Grid` keeps the last drawn frame and sends only
  the changed cells, with a cursor move wherever there is a gap
- **Log pane:** `LogPane` holds up to 1000 lines of up to 1500 characters.
  `\n` starts a new line and `\r` is ignored. The newest lines sit at the
  bottom of the pane
- **Scrolling:** `scroll_up` / `scroll_down` move through the history. Scroll
  stops when the oldest line reaches the top row, and a scrolled view keeps
  showing the same lines while new ones arrive

## Example

```rust
use terminal_renderer as tr;

let mut display = tr::Screen::new()?;
display.setup_screen()?;

let log = display.add_pane(tr::LogPane::new(0, 0, display.columns, display.rows - 1)?);
display.pane(log).push_line("Hello");
display.draw()?;

display.teardown()?;
```

Run the full demo with:

```bash
cargo run --example demo
```

## Design

- **Panes own their content, the screen owns the terminal.** A pane only
  fills its `Grid` when `frame()` is called. `Screen` decides when to draw,
  and is the only part that writes to stdout or changes terminal modes
- **The library renders, the application decides.** The crate will own raw
  terminal setup (echo off, line buffering off, restore on exit). Key
  handling and which pane gets the input stay with the application using
  the crate
- **Storage is separate from display.** `LogHistory` stores lines and the
  scroll position. `LogPane` turns them into a grid. Each can be tested on
  its own
- **Invariants live where the data lives.** Scroll limits are enforced in
  `LogHistory::set_scroll_offset`, so every path that changes the scroll
  (user scroll, new lines, trimming) goes through the same check
- **Small public surface.** Fields are private, and the public API is
  re-exported from `lib.rs`, so callers write `tr::Grid`, not
  `tr::grid::Grid`
- **No dependencies.** Escape sequences are written by hand, to learn what
  libraries such as crossterm and ratatui do underneath

## Testing

```bash
cargo test
```

- Unit tests sit at the foot of each file, in modules named after the
  function they test, so `cargo test scroll` runs only the scroll tests
- Every bug fix starts with a test that is seen to fail first
- Tests assert the behaviour the code should have, using literal expected
  values, and compare drawn rows as plain strings

## To do

**Testing**
- [ ] Expand testing to the rest of the system: `Grid`, `Screen`, `Cell`
- [ ] Test that a shorter line leaves no tail of the longer line it replaces

**Rendering**
- [ ] Draw blank cells as `' '`: a NUL cell does not overwrite old text on
      the real screen
- [ ] Render `Style` (colour, bold, underline), grouping cells that share a
      style into one escape sequence
- [ ] Redraw on terminal resize
- [ ] Optional pane borders, drawn with Unicode box characters (`┌─┐│└┘`)
      and an optional title in the top edge
  - [ ] `Grid::set_border(title)`: the grid keeps its outer size but
        `get_grid_size`, `write_cells` and `clear` work on the inside only,
        so panes need no changes
  - [ ] Settable border colour
  - [ ] Show focus on the border (double line `╔═╗║` or a focus colour)

**Input**
- [ ] Stop the terminal echoing typed text onto the screen: raw mode in
      `Screen` (echo and line buffering off), restored on exit and on panic
- [ ] Input pane with focus, so the user can type into it as if it were
      the terminal. With more than one input pane, the main program needs
      telling which one has new input to read
- [ ] Command history

**Log pane**
- [ ] Scroll with PgUp / PgDn and the mouse wheel
- [ ] Search the history
- [ ] Decide how errors are reported (`push_line` still uses `unwrap`)

**Tidy**
- [ ] Remove the deprecated `std::usize` import in `pane.rs`
- [ ] Remove the empty test module in `grid.rs` and the unused
      `helper_print_screen_dims` in `screen.rs`
- [x] Decide whether `LogHistory` needs to be public: crate-private
