//! Origin tracking (D2870, docs/plain-js-dumps.md §2.3): an activation of a plain dump's
//! continuation `K` is read as an activation of the function `K` stands for.
//!
//! A plain dump writes every suspended activation as a continuation `K` with its origin
//! `K.$djs = { f, at, segs, scaf, cmap }`. Once the dump is loaded and runs, the frames
//! are `K`'s. Dumped again as they are, the next continuation would be written from `K`'s
//! text, which holds the previous dump's prefix: chains would grow with every link. So
//! the exporter reads such a frame as one of `f` at the site `K`'s site stands for: `K`'s
//! site maps to `f`'s through the stretches `K` copied from `f` (`segs`) or `K`'s own
//! constructs (`scaf`: the site itself, loop scaffolds, a finally's rest); `K`'s
//! bindings are `f`'s by name (`K` declares them as `f` does, nested the same way), the
//! temporaries of `f`'s rewritten statement are `K`'s constants or literals (`cmap`), a
//! loop scaffold's iterable is the saved iterator record again, and `K`'s own `$djs_*`
//! locals are dropped, and with them what only they held. The frame is rebuilt in `f`'s
//! layout, so the rest of the exporter sees an activation of `f`, generated from `f`'s
//! text again.

use alloc::string::String;
use alloc::vec::Vec;

use super::*;
use crate::compiler::{Holds, NamedSlot, Place, Site, SiteKind, Syntax};
use crate::interp::run::suspend;

/// What `K.$djs` holds.
struct Record {
    f: Obj,
    segs: Vec<u32>,
    scaf: Vec<Scaffold>,
    /// A temporary of `f`'s rewritten statement: `K`'s binding or a literal.
    cmap: Vec<(String, CVal)>,
}

/// A construct `K` made: kind, offset in `f`, offset in `K`, and a finally's completion
/// (its kind, the binding of `K` that holds its value).
type Scaffold = (String, u32, u32, Option<(i32, String)>);

enum CVal {
    Binding(String),
    Literal(Value),
}

/// The own data property `name` of `o`.
fn own(h: &Heap, o: Obj, name: &str) -> Option<Value> {
    h.own_keys(o)
        .into_iter()
        .find(|&k| h.atom_symbol(k).is_none() && atom_text(h, k) == name)
        .and_then(|k| h.get_own_property(o, k))
        .and_then(|p| match p {
            Property::Data { value, .. } => Some(value),
            _ => None,
        })
}

/// The elements of a dense array.
fn elements(h: &Heap, v: Value) -> Option<Vec<Value>> {
    let o = v.as_object()?;
    if h.object_class(o) != ObjectClass::Array {
        return None;
    }
    let len = h.array_length(o);
    let el: Ref = h.arena().get(o.offset(), ObjectCell::ELEMENTS);
    let el = el.get();
    Some(
        (0..len)
            .map(|i| el.map_or(Value::UNDEFINED, |el| h.value_array_get(el, i)))
            .collect(),
    )
}

fn number(v: Value) -> Option<u32> {
    let n = v.as_number()?;
    (n >= 0.0 && n <= u32::MAX as f64 && (n as u32) as f64 == n).then_some(n as u32)
}

fn string(h: &Heap, v: Value) -> Option<String> {
    v.as_string().map(|s| text_of(h, s))
}

/// `K.$djs` of function `k`, if it is a continuation.
fn record(h: &Heap, k: Obj) -> Option<Record> {
    let r = own(h, k, "$djs")?.as_object()?;
    let f = own(h, r, "f")?.as_object()?;
    if h.object_class(f) != ObjectClass::Function {
        return None;
    }
    let segs = elements(h, own(h, r, "segs")?)?
        .into_iter()
        .map(number)
        .collect::<Option<Vec<u32>>>()?;
    let mut scaf = Vec::new();
    for e in elements(h, own(h, r, "scaf")?)? {
        let e = elements(h, e)?;
        let kind = string(h, *e.first()?)?;
        let (fo, ko) = (number(*e.get(1)?)?, number(*e.get(2)?)?);
        let fin = match (e.get(3), e.get(4)) {
            (Some(k), Some(b)) => Some((k.as_number()? as i32, string(h, *b)?)),
            _ => None,
        };
        scaf.push_out((kind, fo, ko, fin));
    }
    let mut cmap = Vec::new();
    for e in elements(h, own(h, r, "cmap")?)? {
        let e = elements(h, e)?;
        let slot = string(h, *e.first()?)?;
        let v = *e.get(2)?;
        let c = match number(*e.get(1)?)? {
            0 => CVal::Binding(string(h, v)?),
            _ => CVal::Literal(v),
        };
        cmap.push_out((slot, c));
    }
    Some(Record {
        f,
        segs,
        scaf,
        cmap,
    })
}

/// Offset `k` of `K`'s text in `f`'s, through the stretches `K` copied.
fn to_f(segs: &[u32], k: u32) -> Option<u32> {
    segs.chunks(3)
        .find(|s| s.len() == 3 && s[0] <= k && k < s[0] + s[2])
        .map(|s| s[1] + (k - s[0]))
}

/// The slot name's construct and offset (`call.arg@41`).
fn split(name: &str) -> Option<(&str, u32)> {
    let (c, off) = name.rsplit_once('@')?;
    Some((c, off.parse().ok()?))
}

/// The `$dump.L` wrapper of a loop scaffold: the iterator record it continues.
fn unwrap_loop(h: &Heap, r: Value) -> Option<(Value, Value)> {
    let next = own(h, r.as_object()?, "next")?.as_object()?;
    if h.object_class(next) != ObjectClass::Function {
        return None;
    }
    Some((
        captured_value(h, next, "it")?,
        captured_value(h, next, "next")?,
    ))
}

impl Dumper<'_> {
    /// Activation `s` of a continuation, as an activation of its origin: the frame in
    /// the origin's layout, its site and syntax. `Ok(None)`: not a continuation.
    pub(super) fn origin_frame(
        &mut self,
        s: &suspend::Suspended,
        ksite: &Site,
    ) -> Result<Option<(suspend::Suspended, Site, Rc<Syntax>)>, String> {
        let h = self.h;
        let Some(k) = s.frame.first().and_then(|v| v.as_object()) else {
            return Ok(None);
        };
        let Some(rec) = record(h, k) else {
            return Ok(None);
        };
        let no = |why: &str| Err(cat!("a continuation of an earlier dump: ", why));
        // K's site in f's text: K's own (the site it was written for) or a copied one.
        let at_site = rec
            .scaf
            .iter()
            .find(|r| r.0 == "site" && r.2 == ksite.pos)
            .map(|r| r.1);
        let Some(fpos) = at_site.or_else(|| to_f(&rec.segs, ksite.pos)) else {
            return no("its site is not one of its origin's");
        };
        // f's maps: its template's script compiled again.
        let f = rec.f;
        let t = h.closure_template(f);
        let info = template::debug_info(h, t);
        let (a, b) = (info.src_start, info.src_end);
        let params = usize::from(h.arena().get::<u16>(t, FuncTemplate::PARAMS));
        let kind = func_flags::kind(u32::from(h.arena().get::<u16>(t, FuncTemplate::FLAGS)));
        if kind == func_flags::KIND_ASYNC_BODY {
            return no("its origin is a codemode body (0203)");
        }
        let Some(maps) = self.script_maps(t) else {
            return no("its origin's script cannot be compiled again");
        };
        let Some(m) = maps.iter().find(|m| m.start == a && m.end == b) else {
            return no("its origin has no suspension map");
        };
        let Some(fsite) = m
            .sites
            .iter()
            .find(|x| x.pos == fpos && same_kind(x.kind, ksite.kind))
        else {
            return no("its origin has no site there");
        };
        let mut fsite = fsite.clone();
        let syntax = Rc::new(m.syntax.clone());
        if let Some(u) = &fsite.unsupported {
            return Err(cat!(
                "an activation at ",
                fsite.name,
                ", which dumps cannot resume yet: ",
                u
            ));
        }

        // K's slots by name.
        let khp = s.hp as usize;
        let klp = khp + crate::interp::frame::HEADER as usize;
        let kv = |p: Place| {
            let i = match p {
                Place::Arg(i) => 2 + usize::from(i),
                Place::Local(i) => klp + usize::from(i),
                Place::Stack(j) => klp + ksite.locals.len() + usize::from(j),
            };
            s.frame.get(i).copied().unwrap_or(Value::UNDEFINED)
        };
        let knamed = ksite.named_slots();
        let kslot = |name: &str| knamed.iter().find(|x| x.name == name).map(|x| kv(x.place));
        // The bindings of a name in scope, outermost first (`name` or `name@scope`).
        let base = |x: &NamedSlot| -> String {
            match x.name.rsplit_once('@') {
                Some((n, _)) if matches!(x.holds, Holds::Binding) => String::from(n),
                _ => x.name.clone(),
            }
        };
        let scope_of = |x: &NamedSlot| -> u32 {
            x.name
                .rsplit_once('@')
                .and_then(|(_, s)| s.parse().ok())
                .unwrap_or(0)
        };
        let bindings = |named: &[NamedSlot], name: &str| -> Vec<NamedSlot> {
            let mut v: Vec<NamedSlot> = named
                .iter()
                .filter(|x| matches!(x.holds, Holds::Binding) && base(x) == name)
                .cloned()
                .collect();
            crate::sort::sort_by(&mut v, |p, q| scope_of(p) < scope_of(q));
            v
        };
        let fnamed = fsite.named_slots();

        // The frame in f's layout.
        let hp = 2 + params;
        let lp = hp + crate::interp::frame::HEADER as usize;
        let mut frame = alloc::vec![Value::UNDEFINED; lp + fsite.locals.len() + fsite.stack.len()];
        frame[0] = Value::object(f);
        frame[1] = s.frame.get(1).copied().unwrap_or(Value::UNDEFINED);
        for i in 0..crate::interp::frame::HEADER as usize {
            frame[hp + i] = s.frame.get(khp + i).copied().unwrap_or(Value::UNDEFINED);
        }
        frame[hp + crate::interp::frame::ARGC as usize] = Value::int(params as i32);
        let fidx = |p: Place| match p {
            Place::Arg(i) => 2 + usize::from(i),
            Place::Local(i) => lp + usize::from(i),
            Place::Stack(j) => lp + fsite.locals.len() + usize::from(j),
        };
        for x in &fnamed {
            let v = match x.holds {
                Holds::Binding => {
                    let name = base(x);
                    let kname = if name == "arguments" {
                        String::from("$djs_arguments")
                    } else {
                        name.clone()
                    };
                    let fs = bindings(&fnamed, &name);
                    let ks = bindings(&knamed, &kname);
                    let at = fs.iter().position(|y| y.name == x.name).unwrap_or(0);
                    if ks.len() != fs.len() {
                        if fs.len() == 1 && ks.is_empty() {
                            // Declared later in K (in its TDZ) or never reached.
                            Value::UNINITIALIZED
                        } else {
                            return Err(cat!(
                                "a continuation of an earlier dump: its bindings `",
                                name,
                                "` differ from its origin's"
                            ));
                        }
                    } else {
                        kv(ks[at].place)
                    }
                }
                Holds::Hidden => match kslot(&x.name) {
                    Some(v) => v,
                    None => {
                        return Err(cat!(
                            "a continuation of an earlier dump: no `",
                            x.name,
                            "` in its frame"
                        ));
                    }
                },
                Holds::Temp => match self.origin_temp(&rec, &knamed, &kv, at_site, x) {
                    Some(v) => v,
                    None => {
                        return Err(cat!(
                            "a continuation of an earlier dump: its frame has no value for ",
                            x.name
                        ));
                    }
                },
            };
            frame[fidx(x.place)] = v;
        }
        // K's temporaries the origin's frame does not take: K stopped inside its own
        // rewritten statement at another site, whose state K's constructs hold.
        if at_site.is_none() {
            for x in knamed.iter().filter(|x| matches!(x.holds, Holds::Temp)) {
                let Some((_, off)) = split(&x.name) else {
                    continue;
                };
                let scaffold = rec.scaf.iter().any(|r| r.2 == off);
                if !scaffold && to_f(&rec.segs, off).is_none() {
                    return no("it stopped again inside its rewritten statement (0203)");
                }
            }
        }
        // A binding shared with closures is a box in K's frame: f's map says so too where
        // K's closures (f's, made again) share it.
        for (i, slot) in fsite.locals.iter().enumerate() {
            let v = frame[lp + i];
            if let Some(b) = fsite.boxed.get_mut(i) {
                *b = matches!(
                    slot,
                    crate::compiler::Slot::Binding { .. } | crate::compiler::Slot::Hidden(_)
                ) && v.as_internal().is_some();
            }
        }
        let out = suspend::Suspended {
            kind: s.kind,
            state: s.state,
            frame,
            pc: fsite.resume_pc,
            hp: hp as u32,
            promise: s.promise,
            queue: s.queue.clone(),
        };
        Ok(Some((out, fsite, syntax)))
    }

    /// The value of the origin's temporary `x`.
    fn origin_temp(
        &self,
        rec: &Record,
        knamed: &[NamedSlot],
        kv: &dyn Fn(Place) -> Value,
        at_site: Option<u32>,
        x: &NamedSlot,
    ) -> Option<Value> {
        let h = self.h;
        let (c, foff) = split(&x.name)?;
        let kslot = |name: &str| knamed.iter().find(|y| y.name == name).map(|y| kv(y.place));
        // A temporary of the rewritten statement, while K is still at its site.
        if at_site.is_some()
            && let Some((_, cv)) = rec.cmap.iter().find(|e| e.0 == x.name)
        {
            return match cv {
                CVal::Binding(b) => kslot(b),
                CVal::Literal(v) => Some(*v),
            };
        }
        // A scaffold's: the loop's saved iterator record, a finally's completion.
        for r in rec.scaf.iter().filter(|r| r.1 == foff) {
            match (r.0.as_str(), c) {
                ("for-of", "for-of.iterator") | ("for-of", "for-of.next") => {
                    // (Only while K is inside its scaffold: a later iteration of an
                    // outer loop runs the loop as copied text.)
                    if let Some(w) = kslot(&cat!("for-of.iterator@", r.2)) {
                        let (it, next) = unwrap_loop(h, w)?;
                        return Some(if c == "for-of.iterator" { it } else { next });
                    }
                }
                ("finally", "completion.kind") => {
                    return r.3.as_ref().map(|(k, _)| Value::int(*k));
                }
                ("finally", "completion.value") => {
                    return r.3.as_ref().and_then(|(_, b)| kslot(b));
                }
                _ => {}
            }
        }
        // A construct of the text K copied: the same slot at K's offset.
        knamed.iter().find_map(|y| {
            let (yc, ko) = split(&y.name)?;
            (yc == c && to_f(&rec.segs, ko) == Some(foff)).then(|| kv(y.place))
        })
    }
}

/// Whether sites of kinds `a` (the origin's) and `b` (K's) are the same suspension.
fn same_kind(a: SiteKind, b: SiteKind) -> bool {
    a == b
}
