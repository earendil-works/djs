//! Session images (D2066): a VM image without its builtin prefix.
//!
//! Every VM of a build starts from the same base image, and since 0147 the cells below
//! `Roots::BASE_END` (the builtin prefix) never move (D2064), and nothing but a script
//! that changes a builtin writes them (D2063, D2065). A session image keeps what is the
//! VM's own:
//! - the roots cell;
//! - the words of the prefix that differ from the base (none with frozen builtins), in
//!   offset order: the *prefix diff*;
//! - everything from `BASE_END` on.
//!
//! The base supplies the rest. A raw image and its session image hold the same state,
//! and either can be made from the other with the base: [`session_image`] and
//! [`expand_session`].
//!
//! Layout (little-endian):
//!
//! | offset | field |
//! | --- | --- |
//! | 0 | magic `"DJSs"` |
//! | 4 | format ([`SESSION_FORMAT`]) |
//! | 8 | the base's dependency descriptor, hashed (`ImageDeps::hash`, D2265) |
//! | 12 | `BASE_END` |
//! | 16 | FNV-1a 64 of the base's prefix (`base[..BASE_END]`) |
//! | 24 | the roots cell's size |
//! | 28 | n, the number of differing prefix words |
//! | 32 | n × u32: the words' byte offsets, ascending; then zero padding to 8 |
//! | … | n × u64: the words' values |
//! | … | the roots cell |
//! | … | the image from `BASE_END` on |
//!
//! The encoding is a function of the raw image and the base alone, so equal raw images
//! give equal session images.

use alloc::vec::Vec;

use crate::heap::cells::{Header, Roots};

/// `"DJSs"`, little-endian.
pub const SESSION_MAGIC: u32 = u32::from_le_bytes(*b"DJSs");
/// The session image format version (2: the descriptor hash at 8 instead of the build
/// hash, D2265).
pub const SESSION_FORMAT: u32 = 2;
const HEADER: usize = 32;

/// Why a session image cannot be made or expanded.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SessionError {
    /// Not a session image (magic or format), or truncated.
    BadHeader,
    /// The raw image has no builtin prefix (`Roots::BASE_END` 0).
    NoPrefix,
    /// The base is not this image's base: another dependency descriptor, another prefix
    /// end, or other bytes than the image was made against.
    BaseMismatch,
}

impl SessionError {
    pub fn text(&self) -> &'static str {
        match self {
            SessionError::BadHeader => "not a djs session image of this format",
            SessionError::NoPrefix => "the image has no builtin prefix",
            SessionError::BaseMismatch => "the session image was made against another base",
        }
    }
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// The prefix end, descriptor hash and roots cell size of a raw image (or of the base).
fn prefix_of(image: &[u8]) -> Option<(u32, u32, u32)> {
    let roots = Roots::LAYOUT.min_size as usize;
    if image.len() < roots
        || image[Header::KIND.offset as usize] != crate::heap::CellKind::Roots as u8
    {
        return None;
    }
    let size = u32_at(image, Header::SIZE.offset as usize)?;
    let end = u32_at(image, Roots::BASE_END.offset as usize)?;
    let deps = super::ImageDeps::of(image)?.hash();
    (end as usize <= image.len() && size as usize <= end as usize).then_some((end, deps, size))
}

/// FNV-1a 64 of an image's builtin prefix (`image[..Roots::BASE_END]`): the prefix hash a
/// session image (and a snapshot container) records of its base. None without a prefix.
pub fn prefix_hash(image: &[u8]) -> Option<u64> {
    let (end, _, _) = prefix_of(image)?;
    (end != 0).then(|| super::fnv64(&image[..end as usize]))
}

/// The session image of the raw image `image`, against its base `base` (the image every VM
/// of the build starts from).
pub fn session_image(image: &[u8], base: &[u8]) -> Result<Vec<u8>, SessionError> {
    let (end, deps, roots) = prefix_of(image).ok_or(SessionError::BadHeader)?;
    if end == 0 {
        return Err(SessionError::NoPrefix);
    }
    if prefix_of(base) != Some((end, deps, roots)) || base.len() < end as usize {
        return Err(SessionError::BaseMismatch);
    }
    let (from, to) = (roots as usize, end as usize);
    let mut diff: Vec<u32> = Vec::new();
    // Word by word (both are 8-aligned): a few hundred KB, compared at memcmp speed where
    // they agree.
    let (a, b) = (&image[from..to], &base[from..to]);
    let mut i = 0;
    while i < a.len() {
        // Skip equal 64-byte runs quickly.
        let run = (a.len() - i).min(64);
        if a[i..i + run] == b[i..i + run] {
            i += run;
            continue;
        }
        let stop = i + run;
        while i < stop {
            if a[i..i + 8] != b[i..i + 8] {
                diff.push((from + i) as u32);
            }
            i += 8;
        }
    }
    let n = diff.len();
    let pad = (4 * n) % 8;
    let mut out = Vec::with_capacity(HEADER + 12 * n + pad + roots as usize + image.len() - to);
    out.extend_from_slice(&SESSION_MAGIC.to_le_bytes());
    out.extend_from_slice(&SESSION_FORMAT.to_le_bytes());
    out.extend_from_slice(&deps.to_le_bytes());
    out.extend_from_slice(&end.to_le_bytes());
    out.extend_from_slice(&super::fnv64(&base[..to]).to_le_bytes());
    out.extend_from_slice(&roots.to_le_bytes());
    out.extend_from_slice(&(n as u32).to_le_bytes());
    for &o in &diff {
        out.extend_from_slice(&o.to_le_bytes());
    }
    out.resize(out.len() + pad, 0);
    for &o in &diff {
        out.extend_from_slice(&image[o as usize..o as usize + 8]);
    }
    out.extend_from_slice(&image[..roots as usize]);
    out.extend_from_slice(&image[to..]);
    Ok(out)
}

/// Whether `bytes` start like a session image.
pub fn is_session_image(bytes: &[u8]) -> bool {
    u32_at(bytes, 0) == Some(SESSION_MAGIC)
}

/// The prefix words a session image changes: `(byte offset, value)`, ascending.
pub fn session_diff(session: &[u8]) -> Result<Vec<(u32, u64)>, SessionError> {
    let (n, offsets, values, _) = parts(session)?;
    Ok((0..n)
        .map(|i| {
            (
                u32_at(session, offsets + 4 * i).unwrap(),
                u64_at(session, values + 8 * i).unwrap(),
            )
        })
        .collect())
}

/// The header fields: (n, offsets at, values at, roots cell at), checked against the
/// length.
fn parts(s: &[u8]) -> Result<(usize, usize, usize, usize), SessionError> {
    let bad = SessionError::BadHeader;
    if u32_at(s, 0) != Some(SESSION_MAGIC) || u32_at(s, 4) != Some(SESSION_FORMAT) {
        return Err(bad);
    }
    let n = u32_at(s, 28).ok_or(bad)? as usize;
    let roots = u32_at(s, 24).ok_or(bad)? as usize;
    let offsets = HEADER;
    let values = offsets + 4 * n + (4 * n) % 8;
    let roots_at = values + 8 * n;
    if s.len() < roots_at + roots {
        return Err(bad);
    }
    Ok((n, offsets, values, roots_at))
}

/// The raw image a session image stands for, given its base.
pub fn expand_session(session: &[u8], base: &[u8]) -> Result<Vec<u8>, SessionError> {
    let bad = SessionError::BadHeader;
    let (n, offsets, values, roots_at) = parts(session)?;
    let deps = u32_at(session, 8).ok_or(bad)?;
    let end = u32_at(session, 12).ok_or(bad)?;
    let hash = u64_at(session, 16).ok_or(bad)?;
    let roots = u32_at(session, 24).ok_or(bad)?;
    if prefix_of(base) != Some((end, deps, roots))
        || base.len() < end as usize
        || super::fnv64(&base[..end as usize]) != hash
    {
        return Err(SessionError::BaseMismatch);
    }
    let rest = &session[roots_at + roots as usize..];
    let mut out = Vec::new();
    out.reserve_exact(end as usize + rest.len());
    out.extend_from_slice(&session[roots_at..roots_at + roots as usize]);
    out.extend_from_slice(&base[roots as usize..end as usize]);
    out.extend_from_slice(rest);
    let mut last = 0;
    for i in 0..n {
        let o = u32_at(session, offsets + 4 * i).ok_or(bad)?;
        // Ascending, 8-aligned, inside the prefix past the roots cell.
        if o < roots || o >= end || o % 8 != 0 || (i > 0 && o <= last) {
            return Err(bad);
        }
        last = o;
        out[o as usize..o as usize + 8]
            .copy_from_slice(&session[values + 8 * i..values + 8 * i + 8]);
    }
    Ok(out)
}
