//! Byte-preserving .tbl editor.
//!
//! `parser::parse_tbl` / `serialize_tbl` decode every string (UTF-8, else Windows-1254, else
//! EUC-KR) and always write EUC-KR, so a parse + serialize of a client table is NOT lossless:
//! non-Korean text becomes `&#NNNN;` entities and the file grows by ~15 %. This module keeps the
//! plaintext of every row verbatim and only touches the rows the caller replaces / appends, so an
//! untouched table re-encodes to the exact source bytes.

use anyhow::{bail, Result};
use std::ops::Range;

/// Column type codes of the .tbl format (see `parser::serialize_tbl`).
pub const COL_U32: i32 = 6;
pub const COL_STRING: i32 = 7;

pub struct RawTbl {
    /// Everything before the row count (new-structure header, column count, column types).
    head: Vec<u8>,
    pub types: Vec<i32>,
    pub rows: Vec<Vec<u8>>,
    /// Bytes after the last row (normally empty).
    tail: Vec<u8>,
}

fn rd_i32(d: &[u8], pos: usize) -> Result<i32> {
    match d.get(pos..pos + 4) {
        Some(b) => Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        None => bail!("tbl truncated at {pos}"),
    }
}

fn cell_len(d: &[u8], pos: usize, ty: i32) -> Result<usize> {
    Ok(match ty {
        1 | 2 => 1,
        3 | 4 => 2,
        5 | 6 | 8 => 4,
        9..=11 => 8,
        7 => 4 + rd_i32(d, pos)?.max(0) as usize,
        _ => bail!("unknown column type {ty}"),
    })
}

impl RawTbl {
    pub fn parse(plain: &[u8], new_structure: bool) -> Result<Self> {
        let mut pos = if new_structure { 5 } else { 0 };
        let cols = rd_i32(plain, pos)?;
        if cols <= 0 || cols > 1000 {
            bail!("invalid column count {cols}");
        }
        pos += 4;
        let mut types = Vec::new();
        for _ in 0..cols {
            types.push(rd_i32(plain, pos)?);
            pos += 4;
        }
        let head = plain[..pos].to_vec();
        let count = rd_i32(plain, pos)?;
        pos += 4;
        let mut rows = Vec::with_capacity(count.max(0) as usize);
        for _ in 0..count {
            let start = pos;
            for &t in &types {
                pos += cell_len(plain, pos, t)?;
            }
            if pos > plain.len() {
                bail!("tbl row runs past the end of the file");
            }
            rows.push(plain[start..pos].to_vec());
        }
        Ok(Self { head, types, rows, tail: plain[pos..].to_vec() })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = self.head.clone();
        out.extend_from_slice(&(self.rows.len() as i32).to_le_bytes());
        for r in &self.rows {
            out.extend_from_slice(r);
        }
        out.extend_from_slice(&self.tail);
        out
    }

    /// Value of the first column (must be a u32 id column).
    pub fn id(&self, row: &[u8]) -> Option<u32> {
        if self.types.first() != Some(&COL_U32) {
            return None;
        }
        row.get(..4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Byte range of a cell inside a row.
    pub fn cell(&self, row: &[u8], col: usize) -> Result<Range<usize>> {
        let mut pos = 0;
        for (i, &t) in self.types.iter().enumerate() {
            let len = cell_len(row, pos, t)?;
            if i == col {
                return Ok(pos..pos + len);
            }
            pos += len;
        }
        bail!("column {col} out of range")
    }

    /// Removes every row with this id (first column).
    pub fn remove_id(&mut self, id: u32) {
        if self.types.first() != Some(&COL_U32) {
            return;
        }
        self.rows.retain(|r| {
            r.get(..4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])) != Some(id)
        });
    }
}

/// Little-endian cell writers used to build new rows.
pub fn put_u32(row: &mut Vec<u8>, v: u32) {
    row.extend_from_slice(&v.to_le_bytes());
}

pub fn put_str(row: &mut Vec<u8>, s: &str) {
    debug_assert!(s.is_ascii(), "new client texts must be plain ASCII: {s:?}");
    row.extend_from_slice(&(s.len() as i32).to_le_bytes());
    row.extend_from_slice(s.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut d = vec![0u8; 5]; // new-structure header
        d.extend_from_slice(&2i32.to_le_bytes());
        d.extend_from_slice(&6i32.to_le_bytes());
        d.extend_from_slice(&7i32.to_le_bytes());
        d.extend_from_slice(&2i32.to_le_bytes());
        let rows: [(u32, Vec<u8>); 2] = [(1, vec![0xe7, 0xfd, 0xfe, 0x20, 0x72]), (2, b"two".to_vec())];
        for (id, s) in rows {
            d.extend_from_slice(&id.to_le_bytes());
            d.extend_from_slice(&(s.len() as i32).to_le_bytes());
            d.extend_from_slice(&s);
        }
        d
    }

    #[test]
    fn untouched_table_is_byte_exact_even_with_non_korean_text() {
        let d = sample();
        let t = RawTbl::parse(&d, true).unwrap();
        assert_eq!(t.to_bytes(), d);
    }

    #[test]
    fn replace_and_append_keep_other_rows() {
        let d = sample();
        let mut t = RawTbl::parse(&d, true).unwrap();
        t.remove_id(2);
        let mut r = Vec::new();
        put_u32(&mut r, 2);
        put_str(&mut r, "new");
        t.rows.push(r);
        let out = RawTbl::parse(&t.to_bytes(), true).unwrap();
        assert_eq!(out.rows.len(), 2);
        assert_eq!(out.rows[0], t.rows[0]);
        assert_eq!(out.id(&out.rows[1]), Some(2));
    }
}
