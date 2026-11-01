//! `djs heap`: what is in a heap dump (a raw image, an older snapshot, a snapshot
//! container). Every section reads the image through djs-core's own heap code: the
//! walker and the layout table decide what a reference is, and the image is verified
//! first, so the tool cannot drift from the engine (D2264).

mod diff;
mod graph;
pub mod input;
mod json;
mod report;

use djs_core::heap::Heap;

use self::graph::Graph;
use self::json::J;
use self::report::{Options, Section};

const USAGE: &str = "usage: djs heap [--json] [--top N] [--reference FILE] [--diff FILE|base|none] [--only SECTIONS] <dump>\n\
  <dump>       a heap image (`.snapshot` of the REPL, Vm::bytes), a snapshot container\n\
               (Sandbox.bytes(), pi's snapshot()) or an older DJSS/DJPI snapshot\n\
  --reference  the image a container is compressed against, when it is not this build's\n\
               base: pi's prelude image (node scripts/heap/reference.ts pi-prelude ...)\n\
  --diff       compare with this image instead of the container's reference (default),\n\
               or with this build's base (default for raw images), or not at all\n\
  --only       comma-separated sections: summary,kinds,diff,largest,strings,shapes,functions,session\n\
  --top N      rows per list (default 10)\n\
  --json       one JSON object instead of text";

/// Heap limit for loading a dump: room for a collection of a copy.
fn limit(len: usize) -> usize {
    (len * 4).max(64 << 20)
}

pub fn main(args: &[String]) -> i32 {
    let mut json = false;
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
    match run(
        &path,
        reference.as_deref(),
        diff.as_deref(),
        only.as_deref(),
        &Options { top },
    ) {
        Ok(sections) => {
            if json {
                print!(
                    "{}",
                    J::Obj(sections.into_iter().map(|s| (s.title, s.json)).collect()).render()
                );
            } else {
                for s in sections {
                    println!("== {}", s.title);
                    print!("{}", s.text);
                    println!();
                }
            }
            0
        }
        Err(e) => {
            eprintln!("djs heap: {e}");
            1
        }
    }
}

fn read_file(p: &str) -> Result<Vec<u8>, String> {
    std::fs::read(p).map_err(|e| format!("cannot read {p}: {e}"))
}

fn load_heap(image: Vec<u8>, what: &str) -> Result<Heap, String> {
    let len = image.len();
    Heap::from_bytes(image, limit(len))
        .map_err(|e| format!("{what} does not load ({e}): another djs layout, or damaged"))
}

/// Unwraps, loads and analyzes the dump at `path`.
pub fn run(
    path: &str,
    reference: Option<&str>,
    diff: Option<&str>,
    only: Option<&[String]>,
    o: &Options,
) -> Result<Vec<Section>, String> {
    let given = reference.map(read_file).transpose()?;
    let dump = input::read(read_file(path)?, given)?;
    let heap = load_heap(dump.image.clone(), "the image")?;
    let g = Graph::new(&heap);
    let want = |s: &str| only.is_none_or(|o| o.iter().any(|x| x == s));

    // The image a collection would leave (popped stack slots cleared first, as
    // `Vm::collect` does).
    let collected = want("summary").then(|| {
        let mut copy =
            Heap::from_bytes_trusted(dump.image.clone(), limit(dump.image.len())).ok()?;
        if let Some((s, sp)) = g.stack() {
            let cap = (copy.cell_size(s) - djs_core::heap::cells::ValueArray::LAYOUT.min_size) / 8;
            for k in sp..cap {
                copy.arena_mut().write(
                    djs_core::heap::cells::ValueArray::VALUES.at(s, k),
                    djs_core::value::Value::UNDEFINED,
                );
            }
        }
        copy.collect().ok()?;
        Some(copy.arena().len())
    });

    // What to compare with: --diff, else the container's reference, else this build's
    // base for an image of this build.
    let image_build: u32 = heap.arena().get(0, djs_core::heap::cells::Roots::BUILD);
    let diff_ref: Option<(String, Vec<u8>)> = match diff {
        Some("none") => None,
        Some("base") => Some(("this build's base image".into(), djs_prelude::BASE.to_vec())),
        Some(p) => Some((p.to_string(), input::unwrap_reference(read_file(p)?)?)),
        None => match (&dump.reference, &dump.container) {
            (Some(r), Some(c)) => Some((
                input::reference_name(c.reference_kind).to_string(),
                r.clone(),
            )),
            _ if image_build == djs_core::vm::BUILD_HASH => {
                Some(("the base image".into(), djs_prelude::BASE.to_vec()))
            }
            _ => None,
        },
    };

    let mut out = Vec::new();
    if want("summary") {
        out.push(report::overview(path, &dump, &g, collected.flatten()));
    }
    if want("kinds") {
        out.push(report::kinds(&g));
    }
    if want("diff")
        && let Some((name, bytes)) = diff_ref
    {
        let rheap = load_heap(bytes, "the reference image")?;
        let rg = Graph::new(&rheap);
        let d = diff::diff(&rg, &g);
        out.push(report::diff_section(&name, &rg, &g, &d, o));
    }
    if want("largest") {
        out.push(report::largest(&g, o));
    }
    if want("strings") {
        out.push(report::strings(&g, o));
    }
    if want("shapes") {
        out.push(report::shapes(&g, o));
    }
    if want("functions") {
        out.push(report::functions(&g, o));
    }
    if want("session") {
        out.push(report::session(&g, o));
    }
    Ok(out)
}
