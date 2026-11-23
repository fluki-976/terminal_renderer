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

/// Cells for `text` in the default style, for building lines in tests
#[cfg(test)]
pub(crate) fn cells_from(text: &str) -> Vec<Cell> {
    text.chars().map(|char| Cell { char, ..Default::default() }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_from_string() {

        let cells = cells_from(&"New String");
        let expected = vec![
            Cell {char: 'N', style: Style::default()},
            Cell {char: 'e', style: Style::default()},
            Cell {char: 'w', style: Style::default()},
            Cell {char: ' ', style: Style::default()},
            Cell {char: 'S', style: Style::default()},
            Cell {char: 't', style: Style::default()},
            Cell {char: 'r', style: Style::default()},
            Cell {char: 'i', style: Style::default()},
            Cell {char: 'n', style: Style::default()},
            Cell {char: 'g', style: Style::default()},
        ];
        assert!(cells == expected);
    }
}
