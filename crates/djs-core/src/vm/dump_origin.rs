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

/// A construct `K` made: kind, offset in `f`, offset in `K`, and what else it records:
/// a finally's completion (its kind, the binding of `K` that holds its value), a `for
/// await`'s bindings of its iterator and `next` (`[0, it, next]`), the origin's site name
/// of a site that is not one of `K`'s kind (a close awaited by K's own `await`).
type Scaffold = (String, u32, u32, Option<(i32, String)>, Extra);

#[derive(Default)]
struct Extra {
    next: Option<String>,
    site: Option<String>,
    /// A temporary of the rewritten statement that K keeps in a construct of its own
    /// (`["tmp", fo, ko, "call.arg@424", "array.element"]`): the origin's slot and K's
    /// construct at `ko`.
    tmp: Option<(String, String)>,
}

enum CVal {
    Binding(String),
    Literal(Value),
}

/// The own data property `name` of `o`.
pub(super) fn own(h: &Heap, o: Obj, name: &str) -> Option<Value> {
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
pub(super) fn elements(h: &Heap, v: Value) -> Option<Vec<Value>> {
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

/// The array field `name` of record `r`; a missing field is empty (the writer leaves
/// empty fields out, D2900).
fn list(h: &Heap, r: Obj, name: &str) -> Option<Vec<Value>> {
    own(h, r, name).map_or(Some(Vec::new()), |v| elements(h, v))
}

/// `K.$djs` of function `k`, if it is a continuation.
fn record(h: &Heap, k: Obj) -> Option<Record> {
    let r = own(h, k, "$djs")?.as_object()?;
    let f = own(h, r, "f")?.as_object()?;
    if h.object_class(f) != ObjectClass::Function {
        return None;
    }
    let segs = list(h, r, "segs")?
        .into_iter()
        .map(number)
        .collect::<Option<Vec<u32>>>()?;
    let mut scaf = Vec::new();
    for e in list(h, r, "scaf")? {
        let e = elements(h, e)?;
        let kind = string(h, *e.first()?)?;
        let (fo, ko) = (number(*e.get(1)?)?, number(*e.get(2)?)?);
        let mut extra = Extra::default();
        let fin = match (kind.as_str(), e.get(3), e.get(4)) {
            ("tmp", Some(f), Some(k)) => {
                extra.tmp = Some((string(h, *f)?, string(h, *k)?));
                None
            }
            ("site", Some(n), None) => {
                extra.site = Some(string(h, *n)?);
                None
            }
            (_, Some(k), Some(b)) => Some((k.as_number()? as i32, string(h, *b)?)),
            _ => None,
        };
        if let Some(n) = e.get(5) {
            extra.next = Some(string(h, *n)?);
        }
        scaf.push((kind, fo, ko, fin, extra));
    }
    let mut cmap = Vec::new();
    for e in list(h, r, "cmap")? {
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
pub(super) fn unwrap_loop(h: &Heap, r: Value) -> Option<(Value, Value)> {
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
        let site_rec = rec.scaf.iter().find(|r| r.0 == "site" && r.2 == ksite.pos);
        let at_site = site_rec.map(|r| r.1);
        let site_name = site_rec.and_then(|r| r.4.site.clone());
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
        // Several sites of a kind at one construct (a `for await`'s closes): the one
        // with K's place among K's at its copy.
        let nth = if site_name.is_none() {
            self.nth_site_at(k, ksite)
        } else {
            0
        };
        let Some(maps) = self.script_maps(t) else {
            return no("its origin's script cannot be compiled again");
        };
        let Some(m) = maps.iter().find(|m| m.start == a && m.end == b) else {
            return no("its origin has no suspension map");
        };
        let fsite = m
            .sites
            .iter()
            .filter(|x| match &site_name {
                Some(n) => x.name == *n && x.pos == fpos,
                None => x.pos == fpos && same_kind(x.kind, ksite.kind),
            })
            .nth(nth)
            .cloned();
        let syntax = Rc::new(m.syntax.clone());
        let Some(mut fsite) = fsite else {
            return no("its origin has no site there");
        };
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
                // (`arguments` is K's binding `$djs_arguments`.)
                Holds::Hidden => match kslot(&x.name).or_else(|| {
                    (x.name == "arguments")
                        .then(|| {
                            bindings(&knamed, "$djs_arguments")
                                .first()
                                .map(|y| kv(y.place))
                        })
                        .flatten()
                }) {
                    Some(v) => v,
                    // A class's hidden bindings (its field initializers, keys, private
                    // methods): dead once its statement ran, which K's prefix holds.
                    None if super::flight::class_hidden(&x.name) => Value::UNDEFINED,
                    // The function's `this` kept in a local: K is called with it.
                    None if x.name == "this" => frame[1],
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
                    return Err(cat!(
                        "a continuation of an earlier dump: it stopped again inside its rewritten statement, its ",
                        x.name,
                        " (0203)"
                    ));
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

    /// The place of site `ksite` among the sites of its kind at its offset in `k`'s map.
    fn nth_site_at(&mut self, k: Obj, ksite: &Site) -> usize {
        let h = self.h;
        let t = h.closure_template(k);
        let info = template::debug_info(h, t);
        let Some(maps) = self.script_maps(t) else {
            return 0;
        };
        maps.iter()
            .find(|m| m.start == info.src_start && m.end == info.src_end)
            .and_then(|m| {
                m.sites
                    .iter()
                    .filter(|x| x.pos == ksite.pos && x.kind == ksite.kind)
                    .position(|x| x.name == ksite.name)
            })
            .unwrap_or(0)
    }

    /// The value of the origin's temporary `x`; a completion's kind that names a
    /// statement (a `break` or `continue`, D2733) by its offset in `K` names it in `f`.
    fn origin_temp(
        &self,
        rec: &Record,
        knamed: &[NamedSlot],
        kv: &dyn Fn(Place) -> Value,
        at_site: Option<u32>,
        x: &NamedSlot,
    ) -> Option<Value> {
        let mut v = self.origin_temp_k(rec, knamed, kv, at_site, x)?;
        // A constant of K that closures capture (a pattern's source, read by K's
        // getters) is a box in K's frame; the original's temporary is its value.
        if let Some(b) = v.as_internal()
            && self.h.cell_kind(b) == crate::heap::cells::BoxCell::KIND
        {
            v = self.h.arena().get(b, crate::heap::cells::BoxCell::VALUE);
        }
        let (c, foff) = split(&x.name)?;
        let from_k = !rec
            .scaf
            .iter()
            .any(|r| r.0 == "finally" && r.1 == foff && c == "completion.kind");
        match v.as_int() {
            Some(code) if c == "completion.kind" && code >= 3 && from_k => {
                let ko = ((code - 3) / 2) as u32;
                let fo = rec
                    .scaf
                    .iter()
                    .find(|r| r.2 == ko && r.0 != "site")
                    .map(|r| r.1)
                    .or_else(|| to_f(&rec.segs, ko))?;
                Some(Value::int(3 + 2 * fo as i32 + (code - 3) % 2))
            }
            _ => Some(v),
        }
    }

    fn origin_temp_k(
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
        // A temporary of the rewritten statement evaluated before the site: K's constant.
        let from_cmap = || {
            rec.cmap
                .iter()
                .find(|e| e.0 == x.name)
                .and_then(|(_, cv)| match cv {
                    CVal::Binding(b) => kslot(b),
                    CVal::Literal(v) => Some(*v),
                })
        };
        if at_site.is_some()
            && let Some(v) = from_cmap()
        {
            return Some(v);
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
                ("pattern", "pattern.iterator") | ("pattern", "pattern.next") => {
                    // K's array pattern over the record (`undefined` once done).
                    if let Some(w) = kslot(&cat!("pattern.iterator@", r.2)) {
                        if w.is_undefined() {
                            return Some(w);
                        }
                        let (it, next) = unwrap_loop(h, w)?;
                        return Some(if c == "pattern.iterator" { it } else { next });
                    }
                }
                ("for-in", "for-in.iterator") => {
                    // The dump's key iterator (`$dump.forIn`), in K's loop over it.
                    if let Some(w) = kslot(&cat!("for-of.iterator@", r.2)) {
                        return unwrap_loop(h, w).map(|x| x.0);
                    }
                }
                ("for-await", "for-await.iterator") | ("for-await", "for-await.next") => {
                    // Its record's constants, while K is inside the scaffold.
                    let b = if c == "for-await.iterator" {
                        r.3.as_ref().map(|x| x.1.clone())
                    } else {
                        r.4.next.clone()
                    };
                    if let Some(v) = b.and_then(|b| kslot(&b)) {
                        return Some(v);
                    }
                }
                ("using", "using.record")
                | ("using", "completion.kind")
                | ("using", "completion.value") => {
                    // K's scope that adds the original's resources again (D2876).
                    if let Some(v) = kslot(&cat!(c, "@", r.2)) {
                        return Some(v);
                    }
                }
                ("try", "completion.kind") | ("try", "completion.value") => {
                    // A try statement K wrote around the site: its finally's completion.
                    if let Some(v) = kslot(&cat!(c, "@", r.2)) {
                        return Some(v);
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
        // A construct of the text K copied: the same slot at K's offset. (First: in a
        // later iteration of a loop K runs the original's text again.)
        let copied = knamed.iter().find_map(|y| {
            let (yc, ko) = split(&y.name)?;
            (yc == c && to_f(&rec.segs, ko) == Some(foff)).then(|| kv(y.place))
        });
        if copied.is_some() {
            return copied;
        }
        // K stopped again inside its rewritten statement: what it evaluated after the
        // site is in K's own constructs, what came before in its constants.
        for r in rec.scaf.iter().filter(|r| r.0 == "tmp") {
            if let Some((f, kc)) = &r.4.tmp
                && *f == x.name
                && let Some(v) = kslot(&cat!(kc, "@", r.2))
            {
                return Some(v);
            }
        }
        from_cmap()
    }
}

/// Whether sites of kinds `a` (the origin's) and `b` (K's) are the same suspension.
fn same_kind(a: SiteKind, b: SiteKind) -> bool {
    a == b
}
