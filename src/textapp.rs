//! What a text-mode front end needs from an application.

use crate::app::Input;
use crate::i18n::Lang;
use crate::screen::Screen;

pub trait TextApp {
    fn handle(&mut self, input: Input);
    fn click(&mut self, col: usize, row: usize);
    /// Advances background work by one step; returns true while more work remains.
    fn tick(&mut self) -> bool;
    fn render(&self, screen: &mut Screen);
    /// The mouse wheel: positive lines scroll down, negative up. Nothing happens where there is nothing to scroll.
    fn scroll(&mut self, _lines: isize) {}
    fn lang(&self) -> Lang;
    fn wants_quit(&self) -> bool;
}
