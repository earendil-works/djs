//! The image against its reference image (the base, or pi's prelude image): which cells
//! are the reference's own (unchanged, in place or moved), changed, new or gone. That is
//! what a snapshot compressed against the reference costs (D2260).
//!
//! Cells have no identity but their offset, and a collection slides them, so the two
//! graphs are matched structurally: a breadth-first walk from both roots cells pairs the
//! targets of the same slot. Immutable cells (strings, bytecode, BigInts) pair only with
//! the same content, so a replaced string is new, not changed.

use std::collections::VecDeque;

use djs_core::heap::CellKind;
use djs_core::heap::cells::{Header, Shape};
use djs_core::heap::walk::Slot;

use super::graph::{Graph, NONE};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Status {
    /// The reference's cell, the same bytes at the same offset.
    InPlace,
    /// The reference's cell with the same content, at another offset.
    Moved,
    /// The reference's cell, changed.
    Changed,
    /// Not in the reference.
    New,
}

pub struct Diff {
    pub status: Vec<Status>,
    /// 8-byte words that differ from the matched reference cell (all of a new cell's).
    pub changed_words: Vec<u32>,
    /// Reference cells with no counterpart: freed or replaced.
    pub removed: Vec<u32>,
    /// 8-byte words of the image equal to the reference's at the same offset.
    pub same_words: u64,
    /// The image's cells line up with the reference's (offset, kind, size) up to here.
    pub aligned_prefix: u32,
}

fn slots(g: &Graph, cell: u32) -> Vec<Slot> {
    let mut v = Vec::new();
    g.heap.trace_cell(cell, |s| v.push(s));
    v
}

fn slot_pos(s: Slot) -> (u32, u32) {
    match s {
        Slot::Value(o) => (o, 8),
        Slot::Ref(o) => (o, 4),
    }
}

/// Whether two immutable cells (strings, bytecode, BigInts) have the same content: the
/// bytes after the header, except a string's cached hash (computed lazily).
fn same_content(rg: &Graph, r: u32, hg: &Graph, h: u32, size: u32, string: bool) -> bool {
    let (ra, ha) = (rg.heap.arena(), hg.heap.arena());
    if string {
        ra.slice(r + 8, 4) == ha.slice(h + 8, 4)
            && ra.slice(r + 16, size - 16) == ha.slice(h + 16, size - 16)
    } else {
        ra.slice(r + 8, size - 8) == ha.slice(h + 8, size - 8)
    }
}

/// Whether reference cell `r` and image cell `h` may be the same cell.
fn compatible(rg: &Graph, r: u32, hg: &Graph, h: u32) -> bool {
    let (rc, hc) = (rg.cell(r), hg.cell(h));
    if rc.kind != hc.kind {
        return false;
    }
    match rc.kind {
        CellKind::Object => rg.class(rc.offset) == hg.class(hc.offset),
        CellKind::Str8 | CellKind::Str16 | CellKind::Bytes | CellKind::BigInt => {
            let string = matches!(rc.kind, CellKind::Str8 | CellKind::Str16);
            rc.size == hc.size && same_content(rg, rc.offset, hg, hc.offset, rc.size, string)
        }
        CellKind::Shape => {
            rg.heap.arena().get::<u32>(rc.offset, Shape::COUNT)
                == hg.heap.arena().get::<u32>(hc.offset, Shape::COUNT)
        }
        _ => true,
    }
}

pub fn diff(rg: &Graph, hg: &Graph) -> Diff {
    let n = hg.cells.len();
    let mut img_to_ref = vec![NONE; n];
    let mut ref_to_img = vec![NONE; rg.cells.len()];
    img_to_ref[0] = 0;
    ref_to_img[0] = 0;
    let mut queue = VecDeque::from([(0u32, 0u32)]);
    while let Some((r, h)) = queue.pop_front() {
        let rs = slots(rg, rg.cell(r).offset);
        let hs = slots(hg, hg.cell(h).offset);
        for (a, b) in rs.iter().zip(hs.iter()) {
            let (Some(rt), Some(ht)) = (
                rg.heap.slot_target(*a).and_then(|t| rg.index(t)),
                hg.heap.slot_target(*b).and_then(|t| hg.index(t)),
            ) else {
                continue;
            };
            if ref_to_img[rt as usize] != NONE || img_to_ref[ht as usize] != NONE {
                continue;
            }
            if !compatible(rg, rt, hg, ht) {
                continue;
            }
            ref_to_img[rt as usize] = ht;
            img_to_ref[ht as usize] = rt;
            queue.push_back((rt, ht));
        }
    }
    let mut status = vec![Status::New; n];
    let mut changed_words = vec![0u32; n];
    for h in 0..n {
        let hc = hg.cells[h];
        let r = img_to_ref[h];
        if r == NONE {
            changed_words[h] = hc.size / 8;
            continue;
        }
        let rc = rg.cell(r);
        let words = differing_words(rg, rc.offset, rc.size, hg, hc.offset, hc.size, &img_to_ref);
        changed_words[h] = words;
        status[h] = if words > 0 {
            Status::Changed
        } else if rc.offset == hc.offset && same_bytes(rg, hg, hc.offset, hc.size) {
            Status::InPlace
        } else {
            Status::Moved
        };
    }
    let removed = (0..rg.cells.len() as u32)
        .filter(|&r| ref_to_img[r as usize] == NONE)
        .collect();
    let (ra, ha) = (rg.heap.bytes(), hg.heap.bytes());
    let same_words = ra
        .as_chunks::<8>()
        .0
        .iter()
        .zip(ha.as_chunks::<8>().0)
        .filter(|(x, y)| x == y)
        .count() as u64;
    let mut aligned_prefix = 0;
    for (rc, hc) in rg.cells.iter().zip(hg.cells.iter()) {
        if rc.offset != hc.offset || rc.kind != hc.kind || rc.size != hc.size {
            break;
        }
        aligned_prefix = hc.offset + hc.size;
    }
    Diff {
        status,
        changed_words,
        removed,
        same_words,
        aligned_prefix,
    }
}

fn same_bytes(rg: &Graph, hg: &Graph, off: u32, size: u32) -> bool {
    rg.heap.bytes().get(off as usize..(off + size) as usize)
        == hg.heap.bytes().get(off as usize..(off + size) as usize)
}

/// 8-byte words in which image cell `h` differs from reference cell `r`: references
/// compare through the matching, everything else byte for byte (the GC byte of the
/// header aside). A cell of another size differs everywhere.
fn differing_words(
    rg: &Graph,
    r: u32,
    rsize: u32,
    hg: &Graph,
    h: u32,
    hsize: u32,
    img_to_ref: &[u32],
) -> u32 {
    if rsize != hsize {
        return hsize / 8;
    }
    let rb = rg.heap.arena().slice(r, rsize);
    let hb = hg.heap.arena().slice(h, hsize);
    let mut masked = vec![false; hsize as usize];
    let mut words = std::collections::BTreeSet::new();
    masked[Header::GC.offset as usize] = true;
    let rs = slots(rg, r);
    let hs = slots(hg, h);
    for (a, b) in rs.iter().zip(hs.iter()) {
        let (pa, w) = slot_pos(*a);
        let (pb, _) = slot_pos(*b);
        let rel = (pb - h) as usize;
        debug_assert_eq!((pa - r) as usize, rel);
        masked[rel..rel + w as usize].fill(true);
        let rt = rg.heap.slot_target(*a).and_then(|t| rg.index(t));
        let ht = hg.heap.slot_target(*b).and_then(|t| hg.index(t));
        let same = match (rt, ht) {
            (Some(rt), Some(ht)) => img_to_ref[ht as usize] == rt,
            (None, None) => rb[rel..rel + w as usize] == hb[rel..rel + w as usize],
            _ => false,
        };
        if !same {
            words.insert(rel / 8);
        }
    }
    for i in 0..hsize as usize {
        if !masked[i] && rb[i] != hb[i] {
            words.insert(i / 8);
        }
    }
    words.len() as u32
}
