//! Byte offset to line/column mapping, built on demand (for error messages and the
//! bytecode line tables), so the lexer itself never tracks lines.

use alloc::vec::Vec;

/// The start offsets of all lines of a source.
///
/// Lines end at LF, CR, CRLF, U+2028 and U+2029 (the ECMAScript line terminators).
/// Lines and columns are 1-based; columns count code points, like QuickJS.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineTable {
    starts: Vec<u32>,
}

impl LineTable {
    pub fn new(src: &[u8]) -> Self {
        let mut starts = Vec::new();
        starts.push(0);
        let mut p = 0;
        while p < src.len() {
            match src[p] {
                b'\n' => starts.push((p + 1) as u32),
                b'\r' if src.get(p + 1) != Some(&b'\n') => starts.push((p + 1) as u32),
                // U+2028 / U+2029 are E2 80 A8 / E2 80 A9.
                0xE2 if src.get(p + 1) == Some(&0x80)
                    && matches!(src.get(p + 2), Some(0xA8 | 0xA9)) =>
                {
                    p += 2;
                    starts.push((p + 1) as u32);
                }
                _ => {}
            }
            p += 1;
        }
        LineTable { starts }
    }

    /// The number of lines (at least 1).
    pub fn line_count(&self) -> u32 {
        self.starts.len() as u32
    }

    /// The 1-based line containing byte offset `pos`. Offsets past the end map to the
    /// last line.
    pub fn line(&self, pos: u32) -> u32 {
        self.starts.partition_point(|&s| s <= pos) as u32
    }

    /// The byte offset where 1-based line `line` starts.
    pub fn line_start(&self, line: u32) -> u32 {
        let idx = (line.max(1) - 1) as usize;
        self.starts
            .get(idx)
            .or(self.starts.last())
            .copied()
            .unwrap_or(0)
    }

    /// The 1-based line and column of byte offset `pos` in `src` (the source this table
    /// was built from). The column counts code points from the start of the line.
    pub fn line_col(&self, src: &[u8], pos: u32) -> (u32, u32) {
        let line = self.line(pos);
        let start = self.line_start(line) as usize;
        let end = (pos as usize).min(src.len()).max(start);
        // Count the bytes that start a code point (not UTF-8 continuation bytes).
        let col = src[start..end]
            .iter()
            .filter(|&&b| b & 0xC0 != 0x80)
            .count();
        (line, col as u32 + 1)
    }
}
