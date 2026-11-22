#[rustfmt::skip]
#[derive(Clone, Copy, Default, PartialEq)]
pub enum Color {
    /// The terminal's own default colour (SGR 39 for text, 49 for background)
    #[default]
    Default,
    /// Palette index: 0-15 are the basic and bright colours, 16-255 the
    /// extended palette
    Indexed(u8),
    /// 24-bit true colour as red, green, blue
    Rgb(u8, u8, u8),
}

#[derive(Clone, Copy, Default, PartialEq)]
pub struct Style {
    foreground: Color,
    background: Color,
    /// Bit flags, combined with `|`: BOLD = 1, ITALIC = 2, UNDERLINE = 4,
    /// REVERSE = 8, ...
    mods: u8,
}

/// One character position on screen: 
/// The character and the style it is drawn with
///
/// When rendering, group neighbouring cells that share a style so each style
/// escape sequence is sent once per run, not once per cell
#[derive(Clone, Copy, PartialEq)]
pub struct Cell {
    pub style: Style,
    pub char: char,
}

impl Default for Cell {
    fn default() -> Self {
        Cell { style: Style::default(), char: ' ' }
    }
    
}

