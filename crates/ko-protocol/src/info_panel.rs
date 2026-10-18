//! Info panel entries of the game client's right-top list.
//!
//! The 2625 client already has a native list at the top right of the screen (it sits under the
//! Knight Genie bar: Clan Buff, Cape Bonus, Flash bonuses, Clan Notice, ...). Each entry is a
//! title button; clicking it expands the description below it, clicking it again folds it. The
//! server drives that list with `WIZ_NOTICE` (0x2E), notice type 4 (u16-prefixed strings):
//!
//! ```text
//! WIZ_NOTICE
//!   u8  4              notice type: right-top list entry
//!   u8  1 | 2          1 = add the entry, 2 = remove it
//!   str title          u16 length + bytes
//!   str description    u16 length + bytes ('|' separates the lines)
//! ```
//!
//! The reference server sends the removal with the very same two strings as the add, so this
//! module does too. Nothing here needs a client patch.
//!
//! The texts are admin supplied, so everything that goes on the wire is first run through
//! [`sanitize_title`] / [`sanitize_description`]: printable ASCII only (Turkish letters are
//! transliterated, the client font has no reliable code page for the rest), no control
//! characters, bounded length. Both the game server and the admin panel use these functions, so
//! what the admin saves is exactly what is sent.

use crate::{Opcode, Packet, PacketReader};

/// Notice type of the right-top list inside `WIZ_NOTICE`.
pub const NOTICE_TYPE_RIGHT_TOP: u8 = 4;
/// Sub-type: add an entry.
pub const SUB_ADD: u8 = 1;
/// Sub-type: remove an entry.
pub const SUB_REMOVE: u8 = 2;

/// Longest title in characters (the native button is about 90 px wide).
pub const MAX_TITLE_CHARS: usize = 24;
/// Longest description in characters (the notice area of the game allows 255).
pub const MAX_DESCRIPTION_CHARS: usize = 255;
/// Most lines (`|` separated) of one description.
pub const MAX_DESCRIPTION_LINES: usize = 8;
/// Most entries the server keeps in the list of one player (all sources managed by the panel).
pub const MAX_PANEL_ENTRIES: usize = 12;

/// Transliterates the Turkish letters and the common typographic characters to ASCII; returns
/// `None` for characters that must be dropped.
fn to_ascii(c: char) -> Option<char> {
    match c {
        ' '..='~' => Some(c),
        'ç' => Some('c'),
        'Ç' => Some('C'),
        'ğ' => Some('g'),
        'Ğ' => Some('G'),
        'ı' => Some('i'),
        'İ' => Some('I'),
        'ö' => Some('o'),
        'Ö' => Some('O'),
        'ş' => Some('s'),
        'Ş' => Some('S'),
        'ü' => Some('u'),
        'Ü' => Some('U'),
        '\u{2018}' | '\u{2019}' => Some('\''),
        '\u{201C}' | '\u{201D}' => Some('"'),
        '\u{2013}' | '\u{2014}' => Some('-'),
        '\u{00A0}' => Some(' '),
        _ => None,
    }
}

/// Title of an entry: printable ASCII, no `|`, trimmed, at most [`MAX_TITLE_CHARS`] characters.
/// Returns an empty string when nothing usable is left (the entry must then be skipped).
pub fn sanitize_title(raw: &str) -> String {
    let mut out = String::with_capacity(MAX_TITLE_CHARS);
    for c in raw.chars() {
        let c = match c {
            '\n' | '\r' | '\t' | '|' => ' ',
            other => match to_ascii(other) {
                Some(a) => a,
                None => continue,
            },
        };
        if c == ' ' && (out.is_empty() || out.ends_with(' ')) {
            continue; // no leading / doubled blanks
        }
        out.push(c);
        if out.chars().count() >= MAX_TITLE_CHARS {
            break;
        }
    }
    out.trim_end().to_string()
}

/// Description of an entry: printable ASCII, lines separated by `|` (line breaks of the editor
/// become `|`), at most [`MAX_DESCRIPTION_LINES`] non-empty lines and
/// [`MAX_DESCRIPTION_CHARS`] characters in total. Empty when nothing usable is left.
pub fn sanitize_description(raw: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    let flush = |cur: &mut String, lines: &mut Vec<String>| {
        let t = cur.trim().to_string();
        cur.clear();
        if !t.is_empty() {
            lines.push(t);
        }
    };
    for c in raw.chars() {
        match c {
            '\n' | '\r' | '|' => flush(&mut cur, &mut lines),
            '\t' => cur.push(' '),
            other => {
                if let Some(a) = to_ascii(other) {
                    cur.push(a);
                }
            }
        }
    }
    flush(&mut cur, &mut lines);

    let mut out = String::new();
    for line in lines.into_iter().take(MAX_DESCRIPTION_LINES) {
        let sep = usize::from(!out.is_empty());
        let used = out.chars().count();
        if used + sep >= MAX_DESCRIPTION_CHARS {
            break;
        }
        let room = MAX_DESCRIPTION_CHARS - used - sep;
        let piece: String = line.chars().take(room).collect();
        let piece = piece.trim_end();
        if piece.is_empty() {
            break;
        }
        if sep == 1 {
            out.push('|');
        }
        out.push_str(piece);
    }
    out
}

/// `WIZ_NOTICE` type 4 packet adding `title` / `description` to the right-top list. Both texts
/// are sanitized here as well, so a caller cannot put an overlong or malformed text on the wire.
pub fn build_add_packet(title: &str, description: &str) -> Packet {
    build(SUB_ADD, title, description)
}

/// `WIZ_NOTICE` type 4 packet removing the entry `title` (the description is repeated, as the
/// reference server does).
pub fn build_remove_packet(title: &str, description: &str) -> Packet {
    build(SUB_REMOVE, title, description)
}

fn build(sub: u8, title: &str, description: &str) -> Packet {
    let mut pkt = Packet::new(Opcode::WizNotice as u8);
    pkt.write_u8(NOTICE_TYPE_RIGHT_TOP);
    pkt.write_u8(sub);
    pkt.write_string(&sanitize_title(title));
    pkt.write_string(&sanitize_description(description));
    pkt
}

/// Parses a packet made by [`build_add_packet`] / [`build_remove_packet`]:
/// `(sub, title, description)`. `None` for anything else or a truncated packet.
pub fn parse_packet(pkt: &Packet) -> Option<(u8, String, String)> {
    if pkt.opcode != Opcode::WizNotice as u8 {
        return None;
    }
    let mut r = PacketReader::new(&pkt.data);
    if r.read_u8()? != NOTICE_TYPE_RIGHT_TOP {
        return None;
    }
    let sub = r.read_u8()?;
    if sub != SUB_ADD && sub != SUB_REMOVE {
        return None;
    }
    let title = r.read_string()?;
    let description = r.read_string()?;
    if r.remaining() != 0 {
        return None;
    }
    Some((sub, title, description))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_packet_layout() {
        let pkt = build_add_packet("Discord", "Join us|Link in the website");
        assert_eq!(pkt.opcode, 0x2E);
        assert_eq!(pkt.data[0], 4);
        assert_eq!(pkt.data[1], 1);
        let mut expect = vec![4u8, 1, 7, 0];
        expect.extend_from_slice(b"Discord");
        expect.extend_from_slice(&[27, 0]);
        expect.extend_from_slice(b"Join us|Link in the website");
        assert_eq!(pkt.data, expect);
    }

    #[test]
    fn remove_packet_roundtrip() {
        let pkt = build_remove_packet("CLAN PREMIUM", "Aktif");
        let (sub, t, d) = parse_packet(&pkt).unwrap();
        assert_eq!((sub, t.as_str(), d.as_str()), (2, "CLAN PREMIUM", "Aktif"));
    }

    #[test]
    fn parse_rejects_other_packets() {
        let mut p = Packet::new(Opcode::WizNotice as u8);
        p.write_u8(2);
        p.write_u8(0);
        assert!(parse_packet(&p).is_none());
        let mut p = Packet::new(Opcode::WizNotice as u8);
        p.write_u8(4);
        p.write_u8(9);
        p.write_string("a");
        p.write_string("b");
        assert!(parse_packet(&p).is_none());
        // truncated: the description is missing
        let mut p = Packet::new(Opcode::WizNotice as u8);
        p.write_u8(4);
        p.write_u8(1);
        p.write_string("a");
        assert!(parse_packet(&p).is_none());
        // trailing garbage
        let mut p = build_add_packet("a", "b");
        p.write_u8(0);
        assert!(parse_packet(&p).is_none());
        let other = Packet::new(Opcode::WizChat as u8);
        assert!(parse_packet(&other).is_none());
    }

    #[test]
    fn title_is_bounded_and_ascii() {
        assert_eq!(sanitize_title("  Discord   Kalil kazan!  "), "Discord Kalil kazan!");
        assert_eq!(sanitize_title("WhatsApp Hediyeleri"), "WhatsApp Hediyeleri");
        assert_eq!(sanitize_title("Çılgın Şölen Ğüö"), "Cilgin Solen Guo");
        assert_eq!(sanitize_title("a|b\nc\td"), "a b c d");
        assert_eq!(sanitize_title("\u{1}\u{7f}\u{1F600}"), "");
        let long = "x".repeat(500);
        assert_eq!(sanitize_title(&long).chars().count(), MAX_TITLE_CHARS);
        // a cut never leaves a trailing blank
        let t = sanitize_title("12345678901234567890123 456");
        assert!(!t.ends_with(' '));
        assert!(t.chars().count() <= MAX_TITLE_CHARS);
    }

    #[test]
    fn description_lines_and_limits() {
        assert_eq!(sanitize_description("a\r\nb\n\n c |d"), "a|b|c|d");
        assert_eq!(sanitize_description("   \n\n  "), "");
        let many: String = (0..30).map(|i| format!("line{i}\n")).collect();
        let d = sanitize_description(&many);
        assert_eq!(d.split('|').count(), MAX_DESCRIPTION_LINES);
        let long = "y".repeat(5000);
        assert_eq!(sanitize_description(&long).chars().count(), MAX_DESCRIPTION_CHARS);
        let multi = format!("{}\n{}\n{}", "a".repeat(120), "b".repeat(120), "c".repeat(120));
        let d = sanitize_description(&multi);
        assert!(d.chars().count() <= MAX_DESCRIPTION_CHARS);
        assert!(d.is_ascii());
        assert!(!d.ends_with('|'));
        assert_eq!(sanitize_description("Ödül: 10.000.000 TL"), "Odul: 10.000.000 TL");
    }

    #[test]
    fn builder_never_exceeds_the_bounds() {
        let pkt = build_add_packet(&"T".repeat(10_000), &"D\n".repeat(10_000));
        let (_, t, d) = parse_packet(&pkt).unwrap();
        assert!(t.chars().count() <= MAX_TITLE_CHARS);
        assert!(d.chars().count() <= MAX_DESCRIPTION_CHARS);
        // 2 + 2 + title + 2 + description at most
        assert!(pkt.data.len() <= 2 + 2 + MAX_TITLE_CHARS + 2 + MAX_DESCRIPTION_CHARS);
        let weird = build_add_packet("\u{0}\u{1}\u{2}", "\u{0}");
        let (_, t, d) = parse_packet(&weird).unwrap();
        assert!(t.is_empty() && d.is_empty());
    }

    #[test]
    fn sanitizers_are_idempotent() {
        for s in ["Çok güzel|ödül", "  x  y ", "a\nb", "\u{201C}quoted\u{201D}"] {
            let t = sanitize_title(s);
            assert_eq!(sanitize_title(&t), t);
            let d = sanitize_description(s);
            assert_eq!(sanitize_description(&d), d);
        }
    }
}
