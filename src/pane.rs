#[rustfmt::skip]
use std::any::Any;
use std::marker::PhantomData;

use crate::Grid;

mod history;
pub use history::LogPane;

/*****************************************************************************/
/*                                 PaneId                                    */
/*****************************************************************************/
/// Handle to a pane, returned by [`Screen::add_pane`](crate::Screen::add_pane)
///
/// Pass it to [`Screen::pane`](crate::Screen::pane) to get the pane back as
/// its own type `P`, not as a `dyn Pane`. It is `Copy`, so keep it and reuse
/// it as often as needed
pub struct PaneId<P> {
    /// Position of the pane in [`Screen::panes`](crate::Screen::panes)
    pub index: usize,
    /// Records the pane's type `P` for the compiler. Zero-sized: Lets
    /// `Screen::pane` return a `P`, and stops a `PaneId<Grid>` from
    /// being used to fetch a `LogPane`
    pub _type: PhantomData<P>,
}

/*
 * Hand written clone as #[derive] demands P: copy
 */
impl<P> Clone for PaneId<P> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<P> Copy for PaneId<P> {}

/*****************************************************************************/
/*                                  Pane                                     */
/*****************************************************************************/
pub trait Pane: Any {
    /// Returns the [`Grid`] holding what this pane should look like now.
    ///
    /// Called by [`Screen::draw`](crate::Screen::draw) once per frame. A pane
    /// that keeps its content elsewhere, such as [`LogPane`], copies it into
    /// its grid here before returning it.
    fn frame(&mut self) -> &mut Grid;
}
