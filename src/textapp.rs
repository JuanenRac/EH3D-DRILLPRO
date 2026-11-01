//! What a text-mode front end needs from an application.

use crate::app::Input;
use crate::i18n::Lang;
use crate::screen::Screen;

/// What the field that is being edited allows (for the context menu of the window).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldState {
    /// Part of the text is selected.
    pub selected: bool,
    /// The field has some text.
    pub has_text: bool,
    /// Undo has something to bring back.
    pub can_undo: bool,
}

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
    /// The field being edited, when there is one.
    fn field_state(&self) -> Option<FieldState> {
        None
    }
    /// The selected text of the field being edited.
    fn field_copy(&self) -> Option<String> {
        None
    }
    /// Takes the selected text out of the field being edited.
    fn field_cut(&mut self) -> Option<String> {
        None
    }
    /// Puts text in the field being edited, in place of the selection.
    fn field_paste(&mut self, _text: &str) {}
    fn field_select_all(&mut self) {}
    fn field_undo(&mut self) {}
    /// Whether the window fills the screen (`None` where the application has no such setting).
    fn fullscreen(&self) -> Option<bool> {
        None
    }
    fn set_fullscreen(&mut self, _on: bool) {}
}
