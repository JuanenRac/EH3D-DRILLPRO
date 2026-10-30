//! The connection with the machine, chosen in the Parameter window: a drop-down with the five ways the job can reach the machine (the simulator, the
//! serial port, a USB adapter, the EH3D node, the EH3D server) and, under it, what each one needs: a port, an address and a token, a machine of the server.
//! The window that opens over the Parameter window keeps what is typed in the dialog until OK; the Parameter window applies it, and writes it into the
//! machine file without touching any other line of it.

use crate::classic::edit::{Kind as EditKind, LineEdit};
use crate::classic::texts::CText;
use crate::protocol::connection::{ConnectionConfig, Kind};

/// The ways the job reaches the machine, in the order of the drop-down.
pub(crate) const KINDS: [Kind; 5] = [Kind::Demo, Kind::Rs232, Kind::Usb, Kind::Node, Kind::Server];

/// The name of each way, as the drop-down and the Parameter window show it.
pub(crate) const NAMES: [CText; 5] = [
    CText::ConnNameDemo,
    CText::ConnNameRs232,
    CText::ConnNameUsb,
    CText::ConnNameNode,
    CText::ConnNameServer,
];

/// What each way says about itself under the drop-down.
pub(crate) const NOTES: [CText; 5] = [
    CText::ConnNoteDemo,
    CText::ConnNoteRs232,
    CText::ConnNoteUsb,
    CText::ConnNoteNode,
    CText::ConnNoteServer,
];

/// A place of the window to move to with the keys: the drop-down, one of the fields of the way chosen, and the two buttons.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Item {
    Kind,
    Field(usize),
    Ok,
    Cancel,
}

pub(crate) struct ConnState {
    pub kind: usize,
    /// The port of the serial line, the port of the USB adapter, the address and token of the node, the address, token and machine of the server.
    pub vals: [String; 7],
    /// What was typed differs from the connection of the program and is applied with the Parameter window.
    pub dirty: bool,
    pub open: bool,
    /// The list of the five ways is shown under the drop-down.
    pub list: bool,
    pub focus: usize,
    pub edit: Option<LineEdit>,
    pub error: Option<CText>,
}

impl ConnState {
    pub fn from_config(c: &ConnectionConfig) -> Self {
        ConnState {
            kind: KINDS.iter().position(|k| *k == c.kind).unwrap_or(0),
            vals: [
                c.rs232_port.clone(),
                c.usb_port.clone(),
                c.node_address.clone(),
                c.node_token.clone(),
                c.server_address.clone(),
                c.server_token.clone(),
                c.server_machine.clone(),
            ],
            dirty: false,
            open: false,
            list: false,
            focus: 0,
            edit: None,
            error: None,
        }
    }

    /// The fields the way chosen needs: the text of their label and the place of their value in `vals`.
    pub fn fields(&self) -> &'static [(CText, usize)] {
        match KINDS[self.kind] {
            Kind::Demo => &[],
            Kind::Rs232 => &[(CText::ConnPort, 0)],
            Kind::Usb => &[(CText::ConnPort, 1)],
            Kind::Node => &[(CText::ConnAddress, 2), (CText::ConnToken, 3)],
            Kind::Server => &[
                (CText::ConnAddress, 4),
                (CText::ConnToken, 5),
                (CText::ConnMachine, 6),
            ],
        }
    }

    pub fn ring(&self) -> Vec<Item> {
        let mut r = vec![Item::Kind];
        r.extend((0..self.fields().len()).map(Item::Field));
        r.push(Item::Ok);
        r.push(Item::Cancel);
        r
    }

    pub fn item(&self) -> Item {
        let r = self.ring();
        r[self.focus.min(r.len() - 1)]
    }

    /// The value of the field being edited goes back to its place.
    pub fn commit(&mut self) {
        if let Some(e) = self.edit.take() {
            if let Item::Field(k) = self.item() {
                let at = self.fields()[k].1;
                self.vals[at] = e.text.trim().to_string();
            }
        }
    }

    /// Moves to the place `i` of the ring; a field starts being edited with its text selected, as the fields of the Parameter window do.
    pub fn go(&mut self, i: usize) {
        self.commit();
        self.focus = i.min(self.ring().len() - 1);
        if let Item::Field(k) = self.item() {
            let at = self.fields()[k].1;
            self.edit = Some(LineEdit::new(&self.vals[at], EditKind::Text, 60));
        }
    }

    pub fn step(&mut self, delta: isize) {
        let n = self.ring().len() as isize;
        let next = (self.focus as isize + delta).rem_euclid(n) as usize;
        self.go(next);
    }

    /// Another way of the drop-down; the places of the ring change with it, so the focus stays on the drop-down.
    pub fn set_kind(&mut self, k: usize) {
        self.commit();
        self.kind = k.min(KINDS.len() - 1);
        self.focus = 0;
        self.error = None;
    }

    /// The connection the dialog describes, on top of `base` (so the tuning of the machine and the sounds stay as they are), or what is wrong.
    pub fn config(&self, base: &ConnectionConfig) -> Result<ConnectionConfig, CText> {
        let mut c = base.clone();
        c.kind = KINDS[self.kind];
        c.rs232_port = self.vals[0].trim().to_string();
        c.usb_port = self.vals[1].trim().to_string();
        c.node_address = self.vals[2].trim().to_string();
        c.node_token = self.vals[3].trim().to_string();
        c.server_address = self.vals[4].trim().to_string();
        c.server_token = self.vals[5].trim().to_string();
        c.server_machine = self.vals[6].trim().to_string();
        match c.kind {
            Kind::Demo => {}
            Kind::Rs232 if c.rs232_port.is_empty() => return Err(CText::ConnNeedPort),
            Kind::Usb if c.usb_port.is_empty() => return Err(CText::ConnNeedPort),
            Kind::Node if !valid_address(&c.node_address) => return Err(CText::ConnNeedAddress),
            Kind::Server => {
                if !valid_address(&c.server_address) {
                    return Err(CText::ConnNeedAddress);
                }
                if c.server_machine.is_empty() {
                    return Err(CText::ConnNeedMachine);
                }
            }
            _ => {}
        }
        Ok(c)
    }
}

/// A sentence in at most two lines of `width` cells: it is cut at the last space that fits (at the cell that fits when it has none, as the Chinese and the
/// Japanese).
pub(crate) fn wrap_note(text: &str, width: usize) -> (String, String) {
    let cells = |s: &str| s.chars().map(crate::i18n::cell_width).sum::<usize>();
    if cells(text) <= width {
        return (text.to_string(), String::new());
    }
    let mut used = 0;
    let mut cut = 0;
    let mut last_space = None;
    for (i, ch) in text.char_indices() {
        used += crate::i18n::cell_width(ch);
        if used > width {
            break;
        }
        cut = i + ch.len_utf8();
        if ch == ' ' {
            last_space = Some(i);
        }
    }
    let at = last_space.unwrap_or(cut);
    (
        text[..at].trim_end().to_string(),
        text[at..].trim_start().to_string(),
    )
}

/// `host:port` with a port from 1 to 65535.
pub(crate) fn valid_address(s: &str) -> bool {
    match s.trim().rsplit_once(':') {
        Some((host, port)) => {
            !host.is_empty()
                && !host.contains(char::is_whitespace)
                && port.parse::<u32>().is_ok_and(|p| (1..=65535).contains(&p))
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_way_asks_for_what_it_needs() {
        let base = ConnectionConfig::default();
        let mut c = ConnState::from_config(&base);
        // the simulator needs nothing
        assert!(c.fields().is_empty() && c.config(&base).is_ok());
        // the serial line and the adapter need a port
        c.set_kind(1);
        assert_eq!(c.fields().len(), 1);
        c.vals[0] = " ".into();
        assert_eq!(c.config(&base), Err(CText::ConnNeedPort));
        c.vals[0] = "COM4".into();
        assert_eq!(c.config(&base).unwrap().rs232_port, "COM4");
        // the node needs an address, the server an address and a machine, and each has a token
        c.set_kind(3);
        assert_eq!(c.fields().len(), 2);
        c.vals[2] = "192.168.0.50".into();
        assert_eq!(c.config(&base), Err(CText::ConnNeedAddress));
        c.vals[2] = "192.168.0.50:5800".into();
        c.vals[3] = "secret-token".into();
        let node = c.config(&base).unwrap();
        assert_eq!(
            (
                node.kind,
                node.node_address.as_str(),
                node.node_token.as_str()
            ),
            (Kind::Node, "192.168.0.50:5800", "secret-token")
        );
        c.set_kind(4);
        assert_eq!(c.fields().len(), 3);
        c.vals[4] = "cm5:18200".into();
        assert_eq!(c.config(&base), Err(CText::ConnNeedMachine));
        c.vals[6] = "bench-1".into();
        let server = c.config(&base).unwrap();
        assert_eq!(
            (server.kind, server.server_machine.as_str()),
            (Kind::Server, "bench-1")
        );
        // what the other ways had typed stays where it was
        assert_eq!(server.node_address, "192.168.0.50:5800");
    }

    #[test]
    fn the_keys_move_over_the_places_of_the_way_chosen_and_a_field_is_kept_when_left() {
        let base = ConnectionConfig::default();
        let mut c = ConnState::from_config(&base);
        c.set_kind(3);
        assert_eq!(
            c.ring(),
            vec![
                Item::Kind,
                Item::Field(0),
                Item::Field(1),
                Item::Ok,
                Item::Cancel
            ]
        );
        c.step(1);
        assert_eq!(c.item(), Item::Field(0));
        c.edit.as_mut().unwrap().text = " 10.0.0.9:5800 ".into();
        c.step(1);
        assert_eq!(c.vals[2], "10.0.0.9:5800");
        c.step(-2);
        assert_eq!(c.item(), Item::Kind);
        c.step(-1);
        assert_eq!(c.item(), Item::Cancel);
    }

    #[test]
    fn an_address_is_host_and_port() {
        for ok in ["cm5:18200", "192.168.0.50:5800", "[::1]:80"] {
            assert!(valid_address(ok), "{ok}");
        }
        for bad in ["", "cm5", ":80", "cm5:0", "cm5:70000", "a b:80", "cm5:x"] {
            assert!(!valid_address(bad), "{bad}");
        }
    }
}
