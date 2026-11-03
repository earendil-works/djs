//! The sections of `djs heap`'s report: each one computed once and rendered as text and
//! as JSON.

use std::collections::HashMap;

use djs_core::bytecode::intrinsics::INTRINSICS;
use djs_core::bytecode::template::debug_info;
use djs_core::heap::cells::flags;
use djs_core::heap::cells::{
    EphemeronArray, FuncTemplate, HostCall, Interp, Object, PropValues, Roots, Str8, Str16,
    ValueArray,
};
use djs_core::heap::layout::Ref;
use djs_core::heap::{CellKind, ObjectClass};
use djs_core::interp::RunState;
use djs_core::value::Value;

use super::Input;
use super::diff::{Diff, Status};
use super::graph::Graph;
use super::json::J;

/// One section of the report: `key` names it in `--only` and the JSON output.
pub struct Section {
    pub key: String,
    pub title: String,
    pub text: String,
    pub json: J,
}

pub struct Options {
    /// Rows per list.
    pub top: usize,
    /// The sections to compute, by key; all when `None`.
    pub only: Option<Vec<String>>,
    /// The strings, largest, shapes and functions sections over the whole image instead
    /// of the session's own part (`--all`, D2432).
    pub all: bool,
}

impl Options {
    pub fn wants(&self, key: &str) -> bool {
        self.only
            .as_ref()
            .is_none_or(|o| o.iter().any(|k| k == key))
    }
}

impl Default for Options {
    fn default() -> Self {
        Options {
            top: 10,
            only: None,
            all: false,
        }
    }
}

/// The cells the strings, largest, shapes and functions sections describe (D2432): the
/// session's own part, the cells new or changed against the reference, or the whole
/// image (`--all`, or no reference to compare with). The rest, the reference's part, is
/// one line of totals.
pub struct Scope {
    /// Per cell index: in the session's part. `None` for the whole image.
    pub own: Option<Vec<bool>>,
    /// The reference's name, as in the diff section.
    pub reference: String,
}

impl Scope {
    pub fn whole() -> Scope {
        Scope {
            own: None,
            reference: String::new(),
        }
    }

    fn has(&self, i: usize) -> bool {
        self.own.as_ref().is_none_or(|o| o[i])
    }

    fn is_session(&self) -> bool {
        self.own.is_some()
    }

    /// The section's first line in the session view; empty for the whole image.
    fn intro(&self) -> String {
        if self.is_session() {
            format!(
                "  the session's own part: cells new or changed against {} (--all for the whole image)\n",
                self.reference
            )
        } else {
            String::new()
        }
    }

    /// The line of the reference's totals in the session view; empty for the whole image.
    fn rest(&self, what: String) -> String {
        if self.is_session() {
            format!("  unchanged from the reference: {what}\n")
        } else {
            String::new()
        }
    }

    /// `scope` and, in the session view, `reference_part` for a section's JSON.
    fn json(&self, mut fields: Vec<(&'static str, J)>, rest: J) -> J {
        fields.insert(
            0,
            (
                "scope",
                if self.is_session() { "session" } else { "all" }.into(),
            ),
        );
        fields.push((
            "reference_part",
            if self.is_session() { rest } else { J::Null },
        ));
        J::obj(fields)
    }
}

fn section(key: &str, title: impl Into<String>, text: String, json: J) -> Section {
    Section {
        key: key.into(),
        title: title.into(),
        text,
        json,
    }
}

// ---- Text helpers ----------------------------------------------------------------------

/// A table: the first column left-aligned, the rest right-aligned, except a last column
/// named `path` or `keys` (left, unpadded).
fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let n = headers.len();
    let free_last = matches!(headers[n - 1], "path" | "keys" | "string" | "where");
    let mut w: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
    for r in rows {
        for (i, c) in r.iter().enumerate() {
            w[i] = w[i].max(c.chars().count());
        }
    }
    let mut out = String::new();
    let line = |cells: Vec<&str>, out: &mut String| {
        for (i, c) in cells.iter().enumerate() {
            let pad = w[i].saturating_sub(c.chars().count());
            if i > 0 {
                out.push_str("  ");
            }
            if i == 0 || (free_last && i == n - 1) {
                out.push_str(c);
                if i < n - 1 {
                    out.push_str(&" ".repeat(pad));
                }
            } else {
                out.push_str(&" ".repeat(pad));
                out.push_str(c);
            }
        }
        out.push('\n');
    };
    line(headers.to_vec(), &mut out);
    for r in rows {
        line(r.iter().map(String::as_str).collect(), &mut out);
    }
    out.lines()
        .map(|l| format!("  {}\n", l.trim_end()))
        .collect()
}

fn pct(part: u64, whole: u64) -> String {
    if whole == 0 {
        "0%".into()
    } else {
        format!("{:.1}%", part as f64 * 100.0 / whole as f64)
    }
}

pub fn kb(n: usize) -> String {
    format!("{:.1}", n as f64 / 1024.0)
}

fn kbu(n: u64) -> String {
    kb(n as usize)
}

/// Counts and bytes by a key, largest first.
#[derive(Default)]
struct Tally {
    rows: HashMap<String, (u64, u64)>,
}

impl Tally {
    fn add(&mut self, key: impl Into<String>, bytes: u64) {
        let e = self.rows.entry(key.into()).or_default();
        e.0 += 1;
        e.1 += bytes;
    }

    fn sorted(&self) -> Vec<(String, u64, u64)> {
        let mut v: Vec<(String, u64, u64)> = self
            .rows
            .iter()
            .map(|(k, &(n, b))| (k.clone(), n, b))
            .collect();
        v.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
        v
    }

    fn json(&self) -> J {
        J::Arr(
            self.sorted()
                .into_iter()
                .map(|(k, n, b)| {
                    J::obj(vec![
                        ("name", k.into()),
                        ("cells", n.into()),
                        ("bytes", b.into()),
                    ])
                })
                .collect(),
        )
    }
}

// ---- Sections ---------------------------------------------------------------------------

/// A descriptor component's JSON key (js/src/snapshot.ts `IMAGE_DEPS`).
fn dep_key(d: djs_core::vm::ImageDep) -> &'static str {
    use djs_core::vm::ImageDep::*;
    match d {
        Layout => "layout",
        Features => "features",
        Compat => "compat",
        Ops => "ops",
        Intrinsics => "intrinsics",
        Records => "records",
    }
}

pub fn overview(
    input: &Input,
    g: &Graph,
    collected: Option<u32>,
    prefix_words: Option<usize>,
) -> Section {
    let path = input.name;
    let a = g.heap.arena();
    let len = a.len() as u64;
    // The image's dependency descriptor (D2265), against this djs's.
    let deps = djs_core::vm::ImageDeps::of(a.bytes());
    let differs = deps.and_then(|d| d.first_difference(&djs_core::vm::IMAGE_DEPS));
    let old_end: u32 = a.get(0, Roots::OLD_END);
    let live: u64 = g
        .cells
        .iter()
        .zip(&g.live)
        .filter(|(_, l)| **l)
        .map(|(c, _)| c.size as u64)
        .sum();
    let (mut old_cells, mut old_bytes) = (0u64, 0u64);
    for c in &g.cells {
        if c.offset < old_end {
            old_cells += 1;
            old_bytes += c.size as u64;
        }
    }
    let mut t = String::new();
    t.push_str(&format!(
        "  file: {path} ({} KB): {}\n",
        kb(input.file_len),
        input.format
    ));
    t.push_str(&format!(
        "  image: {} KB in {} cells, {}\n",
        kbu(len),
        g.cells.len(),
        match differs {
            None => "dependencies as this djs's".to_string(),
            Some((d, found, ours)) => format!(
                "another {} than this djs ({}; this djs: {}): names of builtins may be off",
                d.name(),
                d.show(found),
                d.show(ours)
            ),
        }
    ));
    t.push_str(&format!(
        "  live: {} KB ({}); garbage: {} KB{}\n",
        kbu(live),
        pct(live, len),
        kbu(len - live),
        collected
            .map(|c| format!("; a collection leaves {} KB", kbu(c as u64)))
            .unwrap_or_default()
    ));
    t.push_str(&format!(
        "  old region: {} KB in {old_cells} cells (OLD_END {old_end}); young: {} KB in {} cells\n",
        kbu(old_bytes),
        kbu(len - old_bytes),
        g.cells.len() as u64 - old_cells
    ));
    let base_end = g.heap.base_end();
    if let Some(n) = prefix_words {
        t.push_str(&format!(
            "  own part: {} KB from BASE_END {base_end} on; {n} words of the builtin prefix changed\n",
            kbu(len - base_end as u64)
        ));
    }
    let json = J::obj(vec![
        ("file", path.into()),
        ("format", input.format.clone().into()),
        ("file_bytes", input.file_len.into()),
        ("image_bytes", len.into()),
        ("cells", g.cells.len().into()),
        (
            "deps",
            match deps {
                Some(d) => J::obj(
                    djs_core::vm::ImageDep::ALL
                        .iter()
                        .map(|&k| (dep_key(k), format!("{:08x}", d.get(k)).into()))
                        .collect(),
                ),
                None => J::Null,
            },
        ),
        ("runs_here", differs.is_none().into()),
        ("live_bytes", live.into()),
        ("garbage_bytes", (len - live).into()),
        ("collected_bytes", collected.into()),
        ("old_end", old_end.into()),
        ("old_cells", old_cells.into()),
        ("old_bytes", old_bytes.into()),
        ("base_end", base_end.into()),
        ("own_bytes", (len - base_end as u64).into()),
        ("prefix_words", prefix_words.into()),
    ]);
    section("summary", "summary", t, json)
}

pub fn kinds(g: &Graph) -> Section {
    let a = g.heap.arena();
    let old_end: u32 = a.get(0, Roots::OLD_END);
    let mut by_kind: HashMap<CellKind, [u64; 4]> = HashMap::new();
    let mut classes = Tally::default();
    for (i, c) in g.cells.iter().enumerate() {
        let e = by_kind.entry(c.kind).or_default();
        e[0] += 1;
        e[1] += c.size as u64;
        if !g.live[i] {
            e[2] += c.size as u64;
        }
        if c.offset < old_end {
            e[3] += c.size as u64;
        }
        if c.kind == CellKind::Object {
            let name = g
                .class(c.offset)
                .map(|cl| format!("{cl:?}"))
                .unwrap_or("?".into());
            classes.add(name, c.size as u64);
        }
    }
    let mut kinds: Vec<(CellKind, [u64; 4])> = by_kind.into_iter().collect();
    kinds.sort_by(|a, b| b.1[1].cmp(&a.1[1]));
    let rows: Vec<Vec<String>> = kinds
        .iter()
        .map(|(k, v)| {
            vec![
                k.layout().name.to_string(),
                v[0].to_string(),
                kbu(v[1]),
                kbu(v[2]),
                kbu(v[3]),
            ]
        })
        .collect();
    let mut t = table(&["cell kind", "cells", "KB", "garbage KB", "old KB"], &rows);
    t.push('\n');
    let crow: Vec<Vec<String>> = classes
        .sorted()
        .into_iter()
        .map(|(k, n, b)| vec![k, n.to_string(), kbu(b)])
        .collect();
    t.push_str(&table(&["object class", "objects", "KB"], &crow));
    let json = J::obj(vec![
        (
            "kinds",
            J::Arr(
                kinds
                    .iter()
                    .map(|(k, v)| {
                        J::obj(vec![
                            ("kind", k.layout().name.into()),
                            ("cells", v[0].into()),
                            ("bytes", v[1].into()),
                            ("garbage_bytes", v[2].into()),
                            ("old_bytes", v[3].into()),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("classes", classes.json()),
    ]);
    section("kinds", "cells by kind and class", t, json)
}

pub fn diff_section(name: &str, rg: &Graph, g: &Graph, d: &Diff, o: &Options) -> Section {
    let len = g.heap.arena().len() as u64;
    let ref_len = rg.heap.arena().len() as u64;
    let mut totals: HashMap<Status, (u64, u64, u64)> = HashMap::new();
    let mut by_kind = Tally::default();
    for (i, c) in g.cells.iter().enumerate() {
        let s = d.status[i];
        let e = totals.entry(s).or_default();
        e.0 += 1;
        e.1 += c.size as u64;
        e.2 += d.changed_words[i] as u64;
        if matches!(s, Status::New | Status::Changed) {
            let tag = if s == Status::New { "new" } else { "changed" };
            by_kind.add(format!("{tag} {}", g.kind_name(i as u32)), c.size as u64);
        }
    }
    let get = |s| totals.get(&s).copied().unwrap_or_default();
    let mut removed = Tally::default();
    for &r in &d.removed {
        removed.add(rg.kind_name(r), rg.cell(r).size as u64);
    }
    // New subtrees: new cells whose dominator is not new, with the new bytes they retain.
    let mut new_ret = vec![0u64; g.cells.len()];
    let mut new_cells = vec![0u64; g.cells.len()];
    for (i, c) in g.cells.iter().enumerate() {
        if d.status[i] == Status::New && g.live[i] {
            new_ret[i] = c.size as u64;
            new_cells[i] = 1;
        }
    }
    for &v in g.rpo[1..].iter().rev() {
        let p = g.idom[v as usize];
        if d.status[p as usize] == Status::New {
            new_ret[p as usize] += new_ret[v as usize];
            new_cells[p as usize] += new_cells[v as usize];
        }
    }
    let mut subtrees: Vec<u32> = (0..g.cells.len() as u32)
        .filter(|&i| {
            d.status[i as usize] == Status::New
                && g.live[i as usize]
                && d.status[g.idom[i as usize] as usize] != Status::New
        })
        .collect();
    subtrees.sort_by(|&a, &b| new_ret[b as usize].cmp(&new_ret[a as usize]));
    let mut changed: Vec<u32> = (0..g.cells.len() as u32)
        .filter(|&i| d.status[i as usize] == Status::Changed)
        .collect();
    changed.sort_by(|&a, &b| d.changed_words[b as usize].cmp(&d.changed_words[a as usize]));

    let (ip, mv, ch, nw) = (
        get(Status::InPlace),
        get(Status::Moved),
        get(Status::Changed),
        get(Status::New),
    );
    let mut t = String::new();
    t.push_str(&format!(
        "  against {name} ({} KB): {} of the image's 8-byte words are the reference's at the same offset; \
         cells line up for the first {} KB ({})\n",
        kbu(ref_len),
        pct(d.same_words, len / 8),
        kb(d.aligned_prefix as usize),
        pct(d.aligned_prefix as u64, len),
    ));
    let first = d.first_misaligned.map(|r| {
        let (size, kind, path) = (rg.cell(r).size, rg.kind_name(r), rg.path(r));
        t.push_str(&format!(
            "  the first cell out of line is the reference's {kind} of {} KB at {path}: what follows sits elsewhere\n",
            kb(size as usize)
        ));
        J::obj(vec![("kind", kind.into()), ("size", size.into()), ("path", path.into())])
    });
    let moved_refs: u64 = d.moved_words.iter().map(|&w| w as u64 * 8).sum();
    let rows = vec![
        vec![
            "unchanged, in place".into(),
            ip.0.to_string(),
            kbu(ip.1),
            String::new(),
        ],
        vec![
            "unchanged, moved".into(),
            mv.0.to_string(),
            kbu(mv.1),
            kbu(moved_refs),
        ],
        vec!["changed".into(), ch.0.to_string(), kbu(ch.1), kbu(ch.2 * 8)],
        vec!["new".into(), nw.0.to_string(), kbu(nw.1), kbu(nw.2 * 8)],
        vec![
            "gone (the reference's, not here)".into(),
            d.removed.len().to_string(),
            kbu(d.removed.iter().map(|&r| rg.cell(r).size as u64).sum()),
            String::new(),
        ],
    ];
    t.push_str(&table(&["cells", "count", "KB", "differing KB"], &rows));
    t.push_str("  (the differing KB of moved cells are references to cells that moved)\n");
    t.push_str("\n  new and changed, by kind:\n");
    let krows: Vec<Vec<String>> = by_kind
        .sorted()
        .into_iter()
        .take(o.top * 2)
        .map(|(k, n, b)| vec![k, n.to_string(), kbu(b)])
        .collect();
    t.push_str(&table(&["what", "cells", "KB"], &krows));
    t.push_str("\n  largest new subtrees (new cells and the new cells only they keep alive):\n");
    let srows: Vec<Vec<String>> = subtrees
        .iter()
        .take(o.top)
        .map(|&i| {
            vec![
                kbu(new_ret[i as usize]),
                new_cells[i as usize].to_string(),
                g.kind_name(i),
                g.path(i),
            ]
        })
        .collect();
    t.push_str(&table(&["KB", "cells", "kind", "path"], &srows));
    if !changed.is_empty() {
        t.push_str("\n  most changed cells of the reference:\n");
        let crows: Vec<Vec<String>> = changed
            .iter()
            .take(o.top)
            .map(|&i| {
                vec![
                    (d.changed_words[i as usize] * 8).to_string(),
                    g.cell(i).size.to_string(),
                    g.kind_name(i),
                    g.path(i),
                ]
            })
            .collect();
        t.push_str(&table(&["bytes differing", "size", "kind", "path"], &crows));
    }
    if !d.removed.is_empty() {
        t.push_str("\n  gone, by kind:\n");
        let rrows: Vec<Vec<String>> = removed
            .sorted()
            .into_iter()
            .take(o.top)
            .map(|(k, n, b)| vec![k, n.to_string(), kbu(b)])
            .collect();
        t.push_str(&table(&["kind", "cells", "KB"], &rrows));
    }
    let status_json = |s: (u64, u64, u64)| {
        J::obj(vec![
            ("cells", s.0.into()),
            ("bytes", s.1.into()),
            ("differing_bytes", (s.2 * 8).into()),
        ])
    };
    let json = J::obj(vec![
        ("reference", name.into()),
        ("reference_bytes", ref_len.into()),
        ("same_words", d.same_words.into()),
        ("words", (len / 8).into()),
        ("aligned_prefix", d.aligned_prefix.into()),
        ("first_misaligned", first.unwrap_or(J::Null)),
        ("moved_reference_bytes", moved_refs.into()),
        ("in_place", status_json(ip)),
        ("moved", status_json(mv)),
        ("changed", status_json(ch)),
        ("new", status_json(nw)),
        (
            "gone",
            J::obj(vec![
                ("cells", d.removed.len().into()),
                (
                    "bytes",
                    d.removed
                        .iter()
                        .map(|&r| rg.cell(r).size as u64)
                        .sum::<u64>()
                        .into(),
                ),
                ("by_kind", removed.json()),
            ]),
        ),
        ("new_and_changed_by_kind", by_kind.json()),
        (
            "new_subtrees",
            J::Arr(
                subtrees
                    .iter()
                    .take(o.top)
                    .map(|&i| {
                        J::obj(vec![
                            ("bytes", new_ret[i as usize].into()),
                            ("cells", new_cells[i as usize].into()),
                            ("kind", g.kind_name(i).into()),
                            ("path", g.path(i).into()),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "most_changed",
            J::Arr(
                changed
                    .iter()
                    .take(o.top)
                    .map(|&i| {
                        J::obj(vec![
                            ("differing_bytes", (d.changed_words[i as usize] * 8).into()),
                            ("size", g.cell(i).size.into()),
                            ("kind", g.kind_name(i).into()),
                            ("path", g.path(i).into()),
                        ])
                    })
                    .collect(),
            ),
        ),
    ]);
    section("diff", format!("diff against {name}"), t, json)
}

pub fn largest(g: &Graph, o: &Options, scope: &Scope) -> Section {
    let n = g.cells.len();
    let mut by_size: Vec<u32> = (0..n as u32).filter(|&i| scope.has(i as usize)).collect();
    by_size.sort_by_key(|&i| std::cmp::Reverse(g.cell(i).size));
    // Retained sizes, in the session view of the session's bytes only: what a cell keeps
    // alive of the session's part on its own (through any cells, base ones included).
    let retained: Vec<u64> = if scope.is_session() {
        let mut r: Vec<u64> = (0..n)
            .map(|i| {
                if scope.has(i) && g.live[i] {
                    g.cells[i].size as u64
                } else {
                    0
                }
            })
            .collect();
        for &v in g.rpo[1..].iter().rev() {
            r[g.idom[v as usize] as usize] += r[v as usize];
        }
        r
    } else {
        g.retained.clone()
    };
    let mut by_retained: Vec<u32> = (1..n as u32)
        .filter(|&i| g.live[i as usize] && scope.has(i as usize) && retained[i as usize] > 0)
        .collect();
    by_retained.sort_by(|&a, &b| retained[b as usize].cmp(&retained[a as usize]));
    let (mut rest_cells, mut rest_bytes) = (0u64, 0u64);
    for (i, c) in g.cells.iter().enumerate() {
        if !scope.has(i) {
            rest_cells += 1;
            rest_bytes += c.size as u64;
        }
    }
    let what = |i: u32| {
        let c = g.cell(i);
        if matches!(c.kind, CellKind::Str8 | CellKind::Str16) {
            format!("{} {}", g.path(i), g.string_preview(c.offset, 40))
        } else {
            g.path(i)
        }
    };
    let mut t = scope.intro();
    t.push_str(&scope.rest(format!("{rest_cells} cells, {} KB", kbu(rest_bytes))));
    t.push_str("  largest cells:\n");
    let rows: Vec<Vec<String>> = by_size
        .iter()
        .take(o.top)
        .map(|&i| vec![kbu(g.cell(i).size as u64), g.kind_name(i), what(i)])
        .collect();
    t.push_str(&table(&["KB", "kind", "path"], &rows));
    if scope.is_session() {
        t.push_str(
            "\n  largest retained sizes (the session's cells that a cell alone keeps alive, itself included):\n",
        );
    } else {
        t.push_str("\n  largest retained sizes (the cell and everything only it keeps alive):\n");
    }
    let rows: Vec<Vec<String>> = by_retained
        .iter()
        .take(o.top)
        .map(|&i| {
            vec![
                kbu(retained[i as usize]),
                kbu(g.cell(i).size as u64),
                g.kind_name(i),
                g.path(i),
            ]
        })
        .collect();
    t.push_str(&table(&["retained KB", "own KB", "kind", "path"], &rows));
    let item = |i: u32| {
        J::obj(vec![
            ("size", g.cell(i).size.into()),
            ("retained", retained[i as usize].into()),
            ("kind", g.kind_name(i).into()),
            ("path", g.path(i).into()),
        ])
    };
    let json = scope.json(
        vec![
            (
                "largest",
                J::Arr(by_size.iter().take(o.top).map(|&i| item(i)).collect()),
            ),
            (
                "retained",
                J::Arr(by_retained.iter().take(o.top).map(|&i| item(i)).collect()),
            ),
        ],
        J::obj(vec![
            ("cells", rest_cells.into()),
            ("bytes", rest_bytes.into()),
        ]),
    );
    section("largest", "largest cells and retained sizes", t, json)
}

pub fn strings(g: &Graph, o: &Options, scope: &Scope) -> Section {
    let a = g.heap.arena();
    let (mut flat, mut flat_bytes, mut two, mut ropes, mut rope_bytes, mut units) =
        (0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    let (mut rest, mut rest_bytes) = (0u64, 0u64);
    // Equal contents (kind, length, data) over the whole image: (copies in scope, copies
    // outside, size, a copy in scope).
    type Content<'a> = (CellKind, u32, &'a [u8]);
    let mut groups: HashMap<Content, (u64, u64, u32, u32)> = HashMap::new();
    let mut flats: Vec<u32> = Vec::new();
    for (i, c) in g.cells.iter().enumerate() {
        if !g.live[i] {
            continue;
        }
        let ours = scope.has(i);
        match c.kind {
            CellKind::Str8 | CellKind::Str16 => {
                let n: u32 = if c.kind == CellKind::Str8 {
                    a.get(c.offset, Str8::LEN)
                } else {
                    a.get(c.offset, Str16::LEN)
                };
                let data = a.slice(c.offset + 16, c.size - 16);
                let e = groups
                    .entry((c.kind, n, data))
                    .or_insert((0, 0, c.size, i as u32));
                if !ours {
                    e.1 += 1;
                    rest += 1;
                    rest_bytes += c.size as u64;
                    continue;
                }
                if e.0 == 0 {
                    e.3 = i as u32;
                }
                e.0 += 1;
                flat += 1;
                flat_bytes += c.size as u64;
                flats.push(i as u32);
                units += n as u64;
                if c.kind == CellKind::Str16 {
                    two += 1;
                }
            }
            CellKind::Rope if ours => {
                ropes += 1;
                rope_bytes += c.size as u64;
            }
            _ => {}
        }
    }
    // A content stored more than once: the extra copies in scope (all of them when the
    // reference's part has one).
    let mut dups: Vec<(u64, u64, u32, u32)> = groups
        .values()
        .filter(|e| e.0 > 0 && e.0 + e.1 > 1)
        .map(|&(ours, theirs, size, i)| {
            let extra = if theirs > 0 { ours } else { ours - 1 };
            (ours + theirs, extra, size, i)
        })
        .collect();
    dups.sort_by_key(|x| (std::cmp::Reverse(x.1 * x.2 as u64), x.3));
    let wasted: u64 = dups.iter().map(|e| e.1 * e.2 as u64).sum();
    flats.sort_by_key(|&i| (std::cmp::Reverse(g.cell(i).size), i));
    let mut t = scope.intro();
    t.push_str(&scope.rest(format!("{rest} flat strings, {} KB", kbu(rest_bytes))));
    t.push_str(&format!(
        "  {flat} flat strings ({} two-byte), {} KB, {units} code units; {ropes} ropes, {} KB\n  \
         {} contents stored more than once, {} KB in the extra copies{}\n",
        two,
        kbu(flat_bytes),
        kbu(rope_bytes),
        dups.len(),
        kbu(wasted),
        if scope.is_session() {
            " (copies of the reference's strings count whole)"
        } else {
            ""
        }
    ));
    if !dups.is_empty() {
        t.push_str("\n  most wasteful duplicates:\n");
        let rows: Vec<Vec<String>> = dups
            .iter()
            .take(o.top)
            .map(|&(n, extra, size, i)| {
                vec![
                    n.to_string(),
                    kbu(extra * size as u64),
                    g.string_preview(g.cell(i).offset, 60),
                ]
            })
            .collect();
        t.push_str(&table(&["copies", "extra KB", "string"], &rows));
    }
    t.push_str("\n  largest strings:\n");
    let rows: Vec<Vec<String>> = flats
        .iter()
        .take(o.top)
        .map(|&i| {
            vec![
                kbu(g.cell(i).size as u64),
                format!("{} {}", g.string_preview(g.cell(i).offset, 50), g.path(i)),
            ]
        })
        .collect();
    t.push_str(&table(&["KB", "where"], &rows));
    let full = |i: u32| -> J {
        g.heap
            .str_to_string(super::graph::str_at(g.cell(i).offset))
            .chars()
            .take(200)
            .collect::<String>()
            .into()
    };
    let json = scope.json(
        vec![
            ("flat", flat.into()),
            ("two_byte", two.into()),
            ("flat_bytes", flat_bytes.into()),
            ("code_units", units.into()),
            ("ropes", ropes.into()),
            ("rope_bytes", rope_bytes.into()),
            ("duplicated_contents", dups.len().into()),
            ("duplicate_bytes", wasted.into()),
            (
                "duplicates",
                J::Arr(
                    dups.iter()
                        .take(o.top)
                        .map(|&(n, extra, size, i)| {
                            J::obj(vec![
                                ("copies", n.into()),
                                ("extra_copies", extra.into()),
                                ("size", size.into()),
                                ("string", full(i)),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "largest",
                J::Arr(
                    flats
                        .iter()
                        .take(o.top)
                        .map(|&i| {
                            J::obj(vec![
                                ("size", g.cell(i).size.into()),
                                ("preview", full(i)),
                                ("path", g.path(i).into()),
                            ])
                        })
                        .collect(),
                ),
            ),
        ],
        J::obj(vec![
            ("flat", rest.into()),
            ("flat_bytes", rest_bytes.into()),
        ]),
    );
    section("strings", "strings", t, json)
}

pub fn shapes(g: &Graph, o: &Options, scope: &Scope) -> Section {
    let a = g.heap.arena();
    let (mut shapes, mut shape_bytes) = (0u64, 0u64);
    let (mut shaped, mut dict, mut bare) = (0u64, 0u64, 0u64);
    let mut per_shape: HashMap<u32, u64> = HashMap::new();
    let (mut rest_shapes, mut rest_bytes, mut rest_objects) = (0u64, 0u64, 0u64);
    for (i, c) in g.cells.iter().enumerate() {
        if !g.live[i] {
            continue;
        }
        if !scope.has(i) {
            match c.kind {
                CellKind::Shape => {
                    rest_shapes += 1;
                    rest_bytes += c.size as u64;
                }
                CellKind::Object => rest_objects += 1,
                _ => {}
            }
            continue;
        }
        match c.kind {
            CellKind::Shape => {
                shapes += 1;
                shape_bytes += c.size as u64;
            }
            CellKind::Object => {
                let props: Ref = a.get(c.offset, Object::PROPS);
                match props.get() {
                    None => bare += 1,
                    Some(_) if g.heap.cell_flags(c.offset) & flags::OBJ_DICTIONARY != 0 => {
                        dict += 1
                    }
                    Some(p) if g.heap.cell_kind(p) == CellKind::PropValues => {
                        shaped += 1;
                        let s: Ref = a.get(p, PropValues::SHAPE);
                        if let Some(s) = s.get() {
                            *per_shape.entry(s).or_default() += 1;
                        }
                    }
                    Some(_) => dict += 1,
                }
            }
            _ => {}
        }
    }
    let shared = per_shape.values().filter(|&&n| n > 1).count();
    let mut common: Vec<(u32, u64)> = per_shape.into_iter().collect();
    common.sort_by(|x, y| y.1.cmp(&x.1).then(x.0.cmp(&y.0)));
    let keys = |s: u32| {
        let ks: Vec<String> = g
            .shape_keys(s)
            .into_iter()
            .map(|k| g.key_label(k).trim_start_matches('.').to_string())
            .collect();
        let mut out = ks.iter().take(12).cloned().collect::<Vec<_>>().join(", ");
        if ks.len() > 12 {
            out.push_str(&format!(", … ({} keys)", ks.len()));
        }
        out
    };
    let mut t = scope.intro();
    t.push_str(&scope.rest(format!(
        "{rest_shapes} shapes ({} KB), {rest_objects} objects",
        kbu(rest_bytes)
    )));
    t.push_str(&format!(
        "  {shapes} shapes ({} KB), {} of them shared by several objects\n  \
         objects: {shaped} with a shape, {dict} in dictionary mode, {bare} without properties\n\n",
        kbu(shape_bytes),
        shared
    ));
    let rows: Vec<Vec<String>> = common
        .iter()
        .take(o.top)
        .map(|&(s, n)| vec![n.to_string(), keys(s)])
        .collect();
    t.push_str(&table(&["objects", "keys"], &rows));
    let json = scope.json(
        vec![
            ("shapes", shapes.into()),
            ("shape_bytes", shape_bytes.into()),
            ("shared_shapes", shared.into()),
            ("objects_with_shape", shaped.into()),
            ("objects_in_dictionary_mode", dict.into()),
            ("objects_without_properties", bare.into()),
            (
                "common",
                J::Arr(
                    common
                        .iter()
                        .take(o.top)
                        .map(|&(s, n)| {
                            J::obj(vec![
                                ("objects", n.into()),
                                (
                                    "keys",
                                    J::Arr(
                                        g.shape_keys(s)
                                            .into_iter()
                                            .map(|k| {
                                                g.heap
                                                    .atom_name(djs_core::heap::Atom::from_id(k))
                                                    .into()
                                            })
                                            .collect(),
                                    ),
                                ),
                            ])
                        })
                        .collect(),
                ),
            ),
        ],
        J::obj(vec![
            ("shapes", rest_shapes.into()),
            ("shape_bytes", rest_bytes.into()),
            ("objects", rest_objects.into()),
        ]),
    );
    section("shapes", "shapes", t, json)
}

pub fn functions(g: &Graph, o: &Options, scope: &Scope) -> Section {
    let a = g.heap.arena();
    // Templates by script: count, the template cells and their parts, bytecode alone.
    let mut scripts: HashMap<String, [u64; 4]> = HashMap::new();
    let mut closures: HashMap<String, u64> = HashMap::new();
    let mut builtins = Tally::default();
    let (mut hosts, mut boxes, mut box_bytes) = (0u64, 0u64, 0u64);
    // The reference's part: templates with their parts, closures.
    let mut rest: HashMap<String, [u64; 4]> = HashMap::new();
    let mut rest_closures = 0u64;
    let mut pools_seen = std::collections::HashSet::new();
    for (i, c) in g.cells.iter().enumerate() {
        if !g.live[i] {
            continue;
        }
        let ours = scope.has(i);
        match c.kind {
            CellKind::Object if !ours => {
                if g.class(c.offset) == Some(ObjectClass::Function) {
                    rest_closures += 1;
                }
                continue;
            }
            CellKind::BoxCell if !ours => continue,
            CellKind::FuncTemplate => {
                let e = if ours {
                    scripts.entry(g.template_script(c.offset)).or_default()
                } else {
                    rest.entry(String::new()).or_default()
                };
                e[0] += 1;
                e[1] += c.size as u64;
                // The code is inline (D2444); the rest hangs off the template.
                e[2] += a.get::<u32>(c.offset, FuncTemplate::CODE_LEN) as u64;
                let info: Ref = a.get(c.offset, FuncTemplate::INFO);
                let debug = debug_info(g.heap, c.offset);
                // A unit's functions share their constant pool (D2446): counted once.
                let consts: Ref = a.get(c.offset, FuncTemplate::CONSTS);
                if let Some(p) = consts.get()
                    && pools_seen.insert(p)
                {
                    e[1] += g.heap.cell_size(p) as u64;
                }
                for r in [
                    a.get(c.offset, FuncTemplate::IC),
                    info,
                    debug.lines,
                    debug.names,
                ] {
                    if let Some(r) = r.get() {
                        e[1] += g.heap.cell_size(r) as u64;
                    }
                }
            }
            CellKind::Object => match g.class(c.offset) {
                Some(ObjectClass::Function) => {
                    let s = g
                        .closure_template(c.offset)
                        .map(|t| g.template_script(t))
                        .unwrap_or("?".into());
                    *closures.entry(s).or_default() += 1;
                }
                Some(ObjectClass::BuiltinClosure) => {
                    let v: Value = a.read(Object::SLOTS.at(c.offset, 0));
                    let name = v
                        .as_int()
                        .and_then(|id| INTRINSICS.get(id as usize))
                        .map(|x| x.name)
                        .unwrap_or("?");
                    builtins.add(name, c.size as u64);
                }
                Some(ObjectClass::HostFunction) => hosts += 1,
                _ => {}
            },
            CellKind::BoxCell => {
                boxes += 1;
                box_bytes += c.size as u64;
            }
            _ => {}
        }
    }
    for (s, n) in &closures {
        scripts.entry(s.clone()).or_default()[3] = *n;
    }
    let mut rows: Vec<(String, [u64; 4])> = scripts.into_iter().collect();
    rows.sort_by(|x, y| y.1[1].cmp(&x.1[1]));
    let total: [u64; 4] = rows.iter().fold([0; 4], |acc, (_, v)| {
        [acc[0] + v[0], acc[1] + v[1], acc[2] + v[2], acc[3] + v[3]]
    });
    let r = rest.get("").copied().unwrap_or_default();
    let mut t = scope.intro();
    t.push_str(&scope.rest(format!(
        "{} templates, {} KB with their parts; {rest_closures} closures",
        r[0],
        kbu(r[1])
    )));
    t += &format!(
        "  {} templates, {} KB with their parts ({} KB of bytecode); {} closures; {} boxes ({} KB) of captured variables; {} host functions\n\n",
        total[0],
        kbu(total[1]),
        kbu(total[2]),
        total[3],
        boxes,
        kbu(box_bytes),
        hosts
    );
    let trows: Vec<Vec<String>> = rows
        .iter()
        .map(|(s, v)| {
            vec![
                s.clone(),
                v[0].to_string(),
                kbu(v[1]),
                kbu(v[2]),
                v[3].to_string(),
            ]
        })
        .collect();
    if !trows.is_empty() {
        t.push_str(&table(
            &["script", "templates", "KB", "bytecode KB", "closures"],
            &trows,
        ));
    }
    let b = builtins.sorted();
    if !b.is_empty() {
        t.push_str("\n  builtin closures (promise resolvers, Promise.all elements, ...):\n");
        let brows: Vec<Vec<String>> = b
            .iter()
            .take(o.top)
            .map(|(k, n, _)| vec![k.clone(), n.to_string()])
            .collect();
        t.push_str(&table(&["intrinsic", "closures"], &brows));
    }
    let json = scope.json(
        vec![
            ("templates", total[0].into()),
            ("template_bytes", total[1].into()),
            ("bytecode_bytes", total[2].into()),
            ("closures", total[3].into()),
            ("boxes", boxes.into()),
            ("host_functions", hosts.into()),
            (
                "scripts",
                J::Arr(
                    rows.iter()
                        .map(|(s, v)| {
                            J::obj(vec![
                                ("script", s.clone().into()),
                                ("templates", v[0].into()),
                                ("bytes", v[1].into()),
                                ("bytecode_bytes", v[2].into()),
                                ("closures", v[3].into()),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("builtin_closures", builtins.json()),
        ],
        J::obj(vec![
            ("templates", r[0].into()),
            ("template_bytes", r[1].into()),
            ("closures", rest_closures.into()),
        ]),
    );
    section("functions", "functions and bytecode", t, json)
}

/// What a waiting VM holds: its run state, queued work, pending host calls, promises and
/// suspended async functions and generators.
pub fn session(g: &Graph, o: &Options) -> Section {
    let a = g.heap.arena();
    let interp: Ref = a.get(0, Roots::INTERP);
    let Some(it) = interp.get() else {
        return section(
            "session",
            "session: what the VM holds while it waits",
            "  no interpreter state\n".into(),
            J::Null,
        );
    };
    let state = RunState::from_u32(a.get(it, Interp::STATE));
    let sp: u32 = a.get(it, Interp::SP);
    let queued = |r: Ref, head: u32, per: u32| {
        r.get()
            .map(|c| (a.get::<u32>(c, ValueArray::LEN).saturating_sub(head)) / per)
            .unwrap_or(0)
    };
    let jobs = queued(a.get(0, Roots::JOBS), a.get(it, Interp::JOB_HEAD), 5);
    let inbox = queued(a.get(it, Interp::INBOX), a.get(it, Interp::INBOX_HEAD), 4);
    let rejections: Ref = a.get(it, Interp::REJECTIONS);
    let rejections = rejections
        .get()
        .map(|r| a.get::<u32>(r, ValueArray::LEN))
        .unwrap_or(0);
    let handles = g.heap.handle_count();
    let mut calls = Vec::new();
    let mut promises = [0u64; 3];
    let mut reactions = 0u64;
    let mut activations = Vec::new();
    let mut weak = 0u64;
    for (i, c) in g.cells.iter().enumerate() {
        if !g.live[i] {
            continue;
        }
        match c.kind {
            CellKind::HostCall => {
                let promise: Value = a.get(c.offset, HostCall::PROMISE);
                let args: Ref = a.get(c.offset, HostCall::ARGS);
                calls.push((
                    a.get::<u32>(c.offset, HostCall::ID),
                    a.get::<u32>(c.offset, HostCall::FN),
                    promise.as_object().is_some(),
                    args.get()
                        .map(|r| a.get::<u32>(r, ValueArray::LEN))
                        .unwrap_or(0),
                ));
            }
            CellKind::EphemeronArray => {
                weak += a.get::<u32>(c.offset, EphemeronArray::LEN) as u64 / 2
            }
            CellKind::Object => match g.class(c.offset) {
                Some(ObjectClass::Promise) => {
                    let st = a
                        .read::<Value>(Object::SLOTS.at(c.offset, 0))
                        .as_int()
                        .unwrap_or(0);
                    promises[st.clamp(0, 2) as usize] += 1;
                    let r: Value = a.read(Object::SLOTS.at(c.offset, 2));
                    if let Some(r) = r.ref_offset() {
                        reactions += a.get::<u32>(r, ValueArray::LEN) as u64 / 3;
                    }
                }
                Some(ObjectClass::Generator) => {
                    let slot = |k: u32| a.read::<Value>(Object::SLOTS.at(c.offset, k));
                    let Some(frame) = slot(1).ref_offset() else {
                        continue;
                    };
                    let pc = slot(2).as_int().unwrap_or(0) as u32;
                    let callee: Value = a.read(ValueArray::VALUES.at(frame, 0));
                    let t = callee
                        .as_object()
                        .and_then(|f| g.closure_template(f.offset()));
                    let (name, script, line) = match t {
                        Some(t) => (
                            g.template_name(t),
                            g.template_script(t),
                            g.template_line(t, pc.saturating_sub(1)),
                        ),
                        None => ("?".into(), "?".into(), None),
                    };
                    let kind = match slot(5).as_int() {
                        Some(0) => "generator",
                        Some(1) => "async function",
                        Some(2) => "async generator",
                        _ => "?",
                    };
                    let st = match slot(0).as_int() {
                        Some(0) => "not started",
                        Some(1) | Some(6) => "at a yield",
                        Some(4) => "at an await",
                        Some(5) => "awaiting a return",
                        _ => "?",
                    };
                    let slots: u32 = a.get(frame, ValueArray::LEN);
                    let at = match line {
                        Some((l, col)) => format!("{script}:{l}:{col}"),
                        None => script,
                    };
                    let locals = g.frame_locals(c.offset);
                    activations.push((kind, st, name, at, slots, g.path(i as u32), locals));
                }
                _ => {}
            },
            _ => {}
        }
    }
    let mut t = format!(
        "  run state {state:?}; {sp} value stack slots in use; {handles} handles\n  \
         {jobs} queued jobs, {inbox} host results waiting in the inbox, {} pending host calls, {rejections} unhandled rejections\n  \
         promises: {} pending ({reactions} reactions waiting), {} fulfilled, {} rejected; {weak} weak entries\n",
        calls.len(),
        promises[0],
        promises[1],
        promises[2]
    );
    if !calls.is_empty() {
        t.push('\n');
        let rows: Vec<Vec<String>> = calls
            .iter()
            .map(|&(id, f, asy, n)| {
                vec![
                    id.to_string(),
                    f.to_string(),
                    if asy { "async" } else { "sync (parked)" }.into(),
                    n.to_string(),
                ]
            })
            .collect();
        t.push_str(&table(
            &["host call", "function id", "kind", "arguments"],
            &rows,
        ));
    }
    if !activations.is_empty() {
        t.push_str("\n  suspended activations:\n");
        let rows: Vec<Vec<String>> = activations
            .iter()
            .take(o.top.max(20))
            .map(|(k, s, n, at, slots, p, _)| {
                vec![
                    k.to_string(),
                    s.to_string(),
                    n.clone(),
                    at.clone(),
                    slots.to_string(),
                    p.clone(),
                ]
            })
            .collect();
        t.push_str(&table(
            &["kind", "state", "function", "at", "slots", "path"],
            &rows,
        ));
        for (_, _, n, at, _, _, locals) in activations.iter().take(o.top.max(20)) {
            if !locals.is_empty() {
                let l: Vec<String> = locals.iter().map(|(k, v)| format!("{k} = {v}")).collect();
                t.push_str(&format!("  {n} at {at} holds: {}\n", l.join(", ")));
            }
        }
    }
    let json = J::obj(vec![
        ("run_state", format!("{state:?}").into()),
        ("stack_slots", sp.into()),
        ("handles", handles.into()),
        ("jobs", jobs.into()),
        ("inbox", inbox.into()),
        ("unhandled_rejections", rejections.into()),
        ("weak_entries", weak.into()),
        (
            "host_calls",
            J::Arr(
                calls
                    .iter()
                    .map(|&(id, f, asy, n)| {
                        J::obj(vec![
                            ("id", id.into()),
                            ("fn", f.into()),
                            ("async", asy.into()),
                            ("arguments", n.into()),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "promises",
            J::obj(vec![
                ("pending", promises[0].into()),
                ("fulfilled", promises[1].into()),
                ("rejected", promises[2].into()),
                ("reactions", reactions.into()),
            ]),
        ),
        (
            "activations",
            J::Arr(
                activations
                    .iter()
                    .map(|(k, s, n, at, slots, p, locals)| {
                        J::obj(vec![
                            ("kind", (*k).into()),
                            ("state", (*s).into()),
                            ("function", n.clone().into()),
                            ("at", at.clone().into()),
                            ("frame_slots", (*slots).into()),
                            ("path", p.clone().into()),
                            (
                                "locals",
                                J::Obj(
                                    locals
                                        .iter()
                                        .map(|(k, v)| (k.clone(), v.clone().into()))
                                        .collect(),
                                ),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
    ]);
    section(
        "session",
        "session: what the VM holds while it waits",
        t,
        json,
    )
}

/// The words of the builtin prefix this image changes against its base (D2066): which
/// builtins a script changed (none with frozen builtins). Cells are named by their path
/// from the builtin registry's names (D2371): `%Math%` for Math's property table.
pub fn prefix(g: &Graph, diff: &[(u32, u64)], o: &Options) -> Section {
    // (cell offset) -> words
    let mut cells: Vec<(u32, u32)> = Vec::new();
    for &(o, _) in diff {
        let i = match g.cells.binary_search_by_key(&o, |c| c.offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let c = g.cells[i].offset;
        match cells.last_mut() {
            Some((last, n)) if *last == c => *n += 1,
            _ => cells.push((c, 1)),
        }
    }
    let rows: Vec<(String, String, u32)> = cells
        .iter()
        .map(|&(c, n)| {
            let i = g.index(c).unwrap();
            let kind = g.cell(i).kind.layout().name.to_string();
            (kind, g.path(i), n)
        })
        .collect();
    let mut text = if diff.is_empty() {
        "  the builtin prefix is the base's: no builtin was changed\n".to_string()
    } else {
        let n =
            |k: usize, one: &str, many: &str| format!("{k} {}", if k == 1 { one } else { many });
        format!(
            "  {} of the builtin prefix {} from the base, in {}\n",
            n(diff.len(), "word", "words"),
            if diff.len() == 1 { "differs" } else { "differ" },
            n(cells.len(), "cell", "cells")
        )
    };
    if !rows.is_empty() {
        let shown: Vec<Vec<String>> = rows
            .iter()
            .take(o.top)
            .map(|(k, p, n)| vec![n.to_string(), k.clone(), p.clone()])
            .collect();
        text.push_str(&table(&["words", "kind", "path"], &shown));
        if rows.len() > o.top {
            text.push_str(&format!("  ... and {} more cells\n", rows.len() - o.top));
        }
    }
    let json = J::obj(vec![
        ("words", (diff.len() as u64).into()),
        (
            "cells",
            J::Arr(
                rows.iter()
                    .map(|(k, p, n)| {
                        J::obj(vec![
                            ("kind", k.as_str().into()),
                            ("path", p.as_str().into()),
                            ("words", (*n as u64).into()),
                        ])
                    })
                    .collect(),
            ),
        ),
    ]);
    section("prefix", "the builtin prefix against the base", text, json)
}
