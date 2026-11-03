//! `djs heap`: what is in a heap dump (a raw image, a session image, a snapshot
//! container). Every section reads the image through djs-core's own heap code: the image
//! is verified on load, the walker and the layout table decide what a reference is and
//! what a field is called, so the tool cannot drift from the engine (D2264).

mod diff;
mod graph;
mod json;
mod report;
#[cfg(test)]
mod tests;

use djs_core::heap::Heap;
use djs_core::heap::cells::ValueArray;
use djs_core::value::Value;

use crate::container;

use self::graph::Graph;
use self::json::J;
pub use self::report::{Options, Section};

const USAGE: &str = "usage: djs heap [--json] [--top N] [--reference FILE] [--diff FILE|base|none] [--only SECTIONS] [--all] <dump>\n\
  <dump>       a heap image (the REPL's `.snapshot --raw`, Vm::bytes), a session image or\n\
               a snapshot container (Sandbox.bytes(), pi's snapshot(), `.snapshot`)\n\
  --reference  the image a container is compressed against, if it is not this build's\n\
               base: pi's prelude image (node scripts/heap/reference.ts pi-prelude ...)\n\
  --diff       compare with this image instead of the container's reference (the\n\
               default; this build's base for a raw image of this build), or not at all\n\
  --only       some of: summary,kinds,diff,prefix,largest,strings,shapes,functions,session\n\
  --all        largest, strings, shapes and functions over the whole image, not only the\n\
               session's own part (the cells new or changed against the reference)\n\
  --top N      rows per list (default 10)\n\
  --json       one JSON object, a key per section";

/// Where an image came from, for the summary.
pub struct Input<'a> {
    pub name: &'a str,
    pub file_len: usize,
    pub format: String,
    pub header: Option<container::Header>,
    pub meta: Option<String>,
}

/// Heap limit for loading a dump: room for a collection of a copy.
fn limit(len: usize) -> usize {
    (len * 4).max(64 << 20)
}

pub fn main(args: &[String]) -> i32 {
    let mut json = false;
    let mut all = false;
    let mut top = 10;
    let mut reference: Option<String> = None;
    let mut diff: Option<String> = None;
    let mut only: Option<Vec<String>> = None;
    let mut path: Option<String> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = |name: &str| it.next().cloned().ok_or(format!("{name} needs a value"));
        let r = match a.as_str() {
            "--json" => {
                json = true;
                Ok(())
            }
            "--all" => {
                all = true;
                Ok(())
            }
            "--top" => value("--top").and_then(|v| {
                v.parse()
                    .map(|n| top = n)
                    .map_err(|_| "--top needs a number".into())
            }),
            "--reference" => value("--reference").map(|v| reference = Some(v)),
            "--diff" => value("--diff").map(|v| diff = Some(v)),
            "--only" => {
                value("--only").map(|v| only = Some(v.split(',').map(str::to_string).collect()))
            }
            s if s.starts_with('-') => Err(format!("unknown option {s}")),
            s => {
                path = Some(s.to_string());
                Ok(())
            }
        };
        if let Err(e) = r {
            eprintln!("djs heap: {e}\n{USAGE}");
            return 2;
        }
    }
    let Some(path) = path else {
        eprintln!("{USAGE}");
        return 2;
    };
    let opts = Options { top, only, all };
    match run(&path, reference.as_deref(), diff.as_deref(), &opts) {
        Ok(sections) => {
            if json {
                print!("{}", to_json(&sections));
            } else {
                print!("{}", to_text(&sections));
            }
            0
        }
        Err(e) => {
            eprintln!("djs heap: {e}");
            1
        }
    }
}

pub fn to_text(sections: &[Section]) -> String {
    sections
        .iter()
        .map(|s| format!("== {}\n{}\n", s.title, s.text))
        .collect()
}

pub fn to_json(sections: &[Section]) -> String {
    J::Obj(
        sections
            .iter()
            .map(|s| (s.key.clone(), s.json.clone()))
            .collect(),
    )
    .render()
}

fn read_file(p: &str) -> Result<Vec<u8>, String> {
    std::fs::read(p).map_err(|e| format!("cannot read {p}: {e}"))
}

/// Reads the dump at `path` and analyzes it.
pub fn run(
    path: &str,
    reference: Option<&str>,
    diff: Option<&str>,
    o: &Options,
) -> Result<Vec<Section>, String> {
    let bytes = read_file(path)?;
    let given = reference.map(read_file).transpose()?;
    let snap = container::read(&bytes, given.as_deref())?;
    let input = Input {
        name: path,
        file_len: bytes.len(),
        format: snap.describe(),
        header: snap.header.clone(),
        meta: snap.meta.clone(),
    };
    // What to compare with: --diff, else the container's reference, else this build's
    // base for an image this build can run (its dependency descriptor, D2265).
    let runs_here = djs_core::vm::ImageDeps::of(&snap.image) == Some(djs_core::vm::IMAGE_DEPS);
    let against: Option<(String, Vec<u8>)> = match diff {
        Some("none") => None,
        Some("base") => Some(("this build's base image".into(), djs_prelude::BASE.to_vec())),
        Some(p) => Some((p.to_string(), container::image_of(&read_file(p)?)?)),
        None => match (&snap.reference, &snap.header) {
            (Some(r), Some(h)) => Some((
                container::reference_name(h.reference_kind).to_string(),
                r.clone(),
            )),
            _ if runs_here => Some(("the base image".into(), djs_prelude::BASE.to_vec())),
            _ => None,
        },
    };
    analyze(
        &snap.image,
        &input,
        against.as_ref().map(|(n, b)| (n.as_str(), b.as_slice())),
        o,
    )
}

/// Analyzes a heap image held in memory (the REPL's VM), against a reference image.
pub fn analyze_image(
    image: &[u8],
    reference: Option<(&str, &[u8])>,
    o: &Options,
) -> Result<Vec<Section>, String> {
    let input = Input {
        name: "(in memory)",
        file_len: image.len(),
        format: "raw image".into(),
        header: None,
        meta: None,
    };
    analyze(image, &input, reference, o)
}

fn load(image: &[u8], what: &str) -> Result<Heap, String> {
    Heap::from_bytes(image.to_vec(), limit(image.len())).map_err(|e| {
        format!("{what} does not load ({e}): an image of another djs layout, or damaged")
    })
}

/// The report on `image`, with a diff against `reference` (name, image) if given.
pub fn analyze(
    image: &[u8],
    input: &Input,
    reference: Option<(&str, &[u8])>,
    o: &Options,
) -> Result<Vec<Section>, String> {
    let heap = load(image, "the image")?;
    let g = Graph::new(&heap);
    // The session image against this build's base (D2066), for an image of this build.
    let session = djs_core::vm::session::session_image(image, djs_prelude::BASE).ok();
    let mut out = Vec::new();
    if o.wants("summary") {
        out.push(report::overview(
            input,
            &g,
            collected_len(image, &g),
            session.as_ref().map(Vec::len),
        ));
    }
    if o.wants("kinds") {
        out.push(report::kinds(&g));
    }
    let scoped = ["largest", "strings", "shapes", "functions"]
        .iter()
        .any(|k| o.wants(k));
    let mut scope = report::Scope::whole();
    if let Some((name, bytes)) = reference
        && (o.wants("diff") || (scoped && !o.all))
    {
        let rheap = load(bytes, "the reference image")?;
        let rg = Graph::new(&rheap);
        let d = diff::diff(&rg, &g);
        if o.wants("diff") {
            out.push(report::diff_section(name, &rg, &g, &d, o));
        }
        if !o.all {
            // The session's own part (D2432).
            scope.own = Some(
                d.status
                    .iter()
                    .map(|s| matches!(s, diff::Status::New | diff::Status::Changed))
                    .collect(),
            );
            scope.reference = name.to_string();
        }
    }
    if o.wants("prefix")
        && let Some(s) = &session
        && let Ok(d) = djs_core::vm::session::session_diff(s)
    {
        out.push(report::prefix(&g, &d, o));
    }
    if o.wants("largest") {
        out.push(report::largest(&g, o, &scope));
    }
    if o.wants("strings") {
        out.push(report::strings(&g, o, &scope));
    }
    if o.wants("shapes") {
        out.push(report::shapes(&g, o, &scope));
    }
    if o.wants("functions") {
        out.push(report::functions(&g, o, &scope));
    }
    if o.wants("session") {
        out.push(report::session(&g, o));
    }
    Ok(out)
}

/// The size a full collection would leave, on a copy (popped stack slots cleared first,
/// as `Vm::collect` does).
fn collected_len(image: &[u8], g: &Graph) -> Option<u32> {
    let mut copy = Heap::from_bytes_trusted(image.to_vec(), limit(image.len())).ok()?;
    if let Some((s, sp)) = g.stack() {
        let cap = (copy.cell_size(s) - ValueArray::LAYOUT.min_size) / 8;
        for k in sp..cap {
            copy.arena_mut()
                .write(ValueArray::VALUES.at(s, k), Value::UNDEFINED);
        }
    }
    copy.collect().ok()?;
    Some(copy.arena().len())
}
