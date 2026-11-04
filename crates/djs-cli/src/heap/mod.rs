//! The REPL's `.heap`: what is in the live VM's heap, against the base image every VM
//! starts from (D2340). Every section reads the image through djs-core's own heap code: the
//! image is verified on load, the walker and the layout table decide what a reference is and
//! what a field is called, so the report cannot drift from the engine (D2264).

mod diff;
mod graph;
pub(crate) mod json;
mod report;
#[cfg(test)]
mod tests;

use djs_core::heap::Heap;
use djs_core::heap::cells::ValueArray;
use djs_core::value::Value;

use self::graph::Graph;
pub use self::report::{Options, Section};

/// Heap limit for loading a copy of the image: room for a collection of it.
fn limit(len: usize) -> usize {
    (len * 4).max(64 << 20)
}

pub fn to_text(sections: &[Section]) -> String {
    sections
        .iter()
        .map(|s| format!("== {}\n{}\n", s.title, s.text))
        .collect()
}

fn load(image: &[u8], what: &str) -> Result<Heap, String> {
    Heap::from_bytes(image.to_vec(), limit(image.len())).map_err(|e| {
        format!("{what} does not load ({e}): an image of another djs layout, or damaged")
    })
}

/// The report on `image` (the live VM's), with a diff against `reference` (name, image)
/// if given.
pub fn analyze(
    image: &[u8],
    reference: Option<(&str, &[u8])>,
    o: &Options,
) -> Result<Vec<Section>, String> {
    let heap = load(image, "the image")?;
    let g = Graph::new(&heap);
    // The builtin prefix's words this image changed against this build's base.
    let prefix = prefix_diff(image, djs_prelude::BASE);
    let mut out = Vec::new();
    if o.wants("summary") {
        out.push(report::overview(
            &g,
            collected_len(image, &g),
            prefix.as_ref().map(Vec::len),
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
        && (o.wants("diff") || scoped)
    {
        let rheap = load(bytes, "the reference image")?;
        let rg = Graph::new(&rheap);
        let d = diff::diff(&rg, &g);
        if o.wants("diff") {
            out.push(report::diff_section(name, &rg, &g, &d, o));
        }
        // The session's own part (D2432).
        scope.own = Some(
            d.status
                .iter()
                .map(|s| matches!(s, diff::Status::New | diff::Status::Changed))
                .collect(),
        );
        scope.reference = name.to_string();
    }
    if o.wants("prefix")
        && let Some(d) = &prefix
    {
        out.push(report::prefix(&g, d, o));
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

/// The words of the builtin prefix (from the roots cell to `Roots::BASE_END`) where
/// `image` differs from `base`, as (byte offset, value), ascending: what a script changed
/// in the builtins (none with frozen builtins). None unless both have the same prefix end
/// and roots cell size.
fn prefix_diff(image: &[u8], base: &[u8]) -> Option<Vec<(u32, u64)>> {
    use djs_core::heap::cells::{Header, Roots};
    let u32_at = |b: &[u8], at: u32| {
        Some(u32::from_le_bytes(
            b.get(at as usize..at as usize + 4)?.try_into().ok()?,
        ))
    };
    let end = u32_at(image, Roots::BASE_END.offset)?;
    let roots = u32_at(image, Header::SIZE.offset)?;
    if end == 0
        || u32_at(base, Roots::BASE_END.offset)? != end
        || u32_at(base, Header::SIZE.offset)? != roots
    {
        return None;
    }
    let (from, to) = (roots as usize, end as usize);
    let (a, b) = (image.get(from..to)?, base.get(from..to)?);
    Some(
        a.as_chunks::<8>()
            .0
            .iter()
            .zip(b.as_chunks::<8>().0)
            .enumerate()
            .filter(|(_, (x, y))| x != y)
            .map(|(i, (x, _))| ((from + 8 * i) as u32, u64::from_le_bytes(*x)))
            .collect(),
    )
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
