//! RegExp leaf intrinsics over `crate::regexp` (D1332): creating RegExp objects,
//! reading their internal slots, the escaped source, and building match results. The
//! matcher itself runs in `interp/run/json_regexp.rs`, which owns fuel and suspension.
//!
//! A RegExp object (`ObjectClass::RegExp`) holds its compiled program in a U32Array cell
//! (pointer-free words, so the heap image carries it as is), its source string and its
//! flag bits. `lastIndex` is an ordinary own data property (writable only).
use crate::PushOut;

use alloc::string::String;
use alloc::vec::Vec;

use crate::heap::{
    Heap, HeapError, ObjectClass, PropFlags, Property, PropertyDescriptor, StrView, atoms,
};
use crate::interp::conv::{Fault, type_error};
use crate::interp::{ErrorKind, Intrinsic, realm};
use crate::regexp::{self, Captures, Flags, Program};
use crate::value::{Obj, Str, Value};

pub(crate) const SLOT_PROGRAM: u32 = 0;
pub(crate) const SLOT_SOURCE: u32 = 1;
pub(crate) const SLOT_FLAGS: u32 = 2;

/// RegExp String Iterator slots and flag bits.
pub(crate) const IT_REGEXP: u32 = 0;
pub(crate) const IT_STRING: u32 = 1;
pub(crate) const IT_FLAGS: u32 = 2;
pub(crate) const IT_GLOBAL: i32 = 1;
pub(crate) const IT_UNICODE: i32 = 2;
pub(crate) const IT_DONE: i32 = 4;

/// The flag letters in `flags` order (hasIndices first, as in RegExp.prototype.flags).
pub(crate) const FLAG_CHARS: [(u8, Flags); 8] = [
    (b'd', Flags::HAS_INDICES),
    (b'g', Flags::GLOBAL),
    (b'i', Flags::IGNORE_CASE),
    (b'm', Flags::MULTILINE),
    (b's', Flags::DOT_ALL),
    (b'u', Flags::UNICODE),
    (b'v', Flags::UNICODE_SETS),
    (b'y', Flags::STICKY),
];

fn syntax_error<T>(msg: &str) -> Result<T, Fault> {
    Err(Fault::error(ErrorKind::SyntaxError, String::from(msg)))
}

/// The RegExp object `v`, if it is one.
pub(crate) fn as_regexp(heap: &Heap, v: Value) -> Option<Obj> {
    v.as_object()
        .filter(|&o| heap.object_class(o) == ObjectClass::RegExp)
}

/// Compiles `pattern` with `flags` (both as written in the source or given to the
/// constructor), with QuickJS's SyntaxError messages.
pub(crate) fn compile(pattern: &[u16], flags: &[u16]) -> Result<Program, regexp::SyntaxError> {
    let f = Flags::parse(flags)?;
    regexp::compile(pattern, f)
}

/// A U32Array cell holding `prog`'s words.
pub(crate) fn program_cell(heap: &mut Heap, prog: &Program) -> Result<u32, HeapError> {
    let words = prog.words();
    let cell = heap.alloc_u32_array(words.len() as u32)?;
    for (i, &w) in words.iter().enumerate() {
        heap.u32_array_set(cell, i as u32, w);
    }
    Ok(cell)
}

/// RegExpCreate: a new RegExp object with prototype `proto` (%RegExp.prototype% if
/// `None`), `lastIndex` 0. `cell` is an already compiled program of `pattern` and
/// `flags` (a regex literal's, D1502).
pub(crate) fn create(
    heap: &mut Heap,
    proto: Option<Obj>,
    pattern: Str,
    flags: Str,
    cell: Option<u32>,
) -> Result<Obj, Fault> {
    let cell = match cell {
        Some(c) => c,
        None => {
            let p = heap.str_to_utf16(pattern);
            let f = heap.str_to_utf16(flags);
            let prog = match compile(&p, &f) {
                Ok(prog) => prog,
                Err(e) => return syntax_error(e.message()),
            };
            program_cell(heap, &prog)?
        }
    };
    // The flag bits are the program's header word (`compiler::H_FLAGS`).
    let bits = heap.u32_array_get(cell, 0);
    let proto = proto.unwrap_or_else(|| realm::intrinsic_obj(heap, Intrinsic::RegExpProto));
    let o = heap.alloc_object(ObjectClass::RegExp, Some(proto), 3)?;
    heap.set_object_slot(o, SLOT_PROGRAM, Value::internal(cell));
    heap.set_object_slot(o, SLOT_SOURCE, Value::string(pattern));
    heap.set_object_slot(o, SLOT_FLAGS, Value::int(bits as i32));
    let key = atoms::LAST_INDEX;
    let d = PropertyDescriptor::data(Value::int(0), PropFlags::WRITABLE);
    heap.define_own_property(o, key, &d)?;
    Ok(o)
}

/// The compiled program of a RegExp object (copied out of its cell).
pub(crate) fn program(heap: &Heap, o: Obj) -> Program {
    let cell = heap
        .object_slot(o, SLOT_PROGRAM)
        .as_internal()
        .expect("RegExp program");
    let n = heap.u32_array_len(cell);
    // One bulk read: this runs on every exec.
    let bytes = heap
        .arena()
        .slice(crate::heap::cells::U32Array::DATA.at(cell, 0), 4 * n);
    let words = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b))
        .collect();
    Program::from_words(words)
}

pub(crate) fn flags(heap: &Heap, o: Obj) -> Flags {
    Flags::from_bits(heap.object_slot(o, SLOT_FLAGS).as_int().unwrap_or(0) as u16)
}

pub(crate) fn source(heap: &Heap, o: Obj) -> Str {
    heap.object_slot(o, SLOT_SOURCE)
        .as_string()
        .expect("RegExp source")
}

/// The flags as a string, in canonical order ([[OriginalFlags]]).
pub(crate) fn flags_string(heap: &mut Heap, o: Obj) -> Result<Str, Fault> {
    let f = flags(heap, o);
    let s: Vec<u8> = FLAG_CHARS
        .iter()
        .filter(|(_, bit)| f.contains(*bit))
        .map(|(c, _)| *c)
        .collect();
    Ok(heap.alloc_latin1(&s)?)
}

/// Whether the RegExp has the flag letter `c`.
pub(crate) fn has_flag(heap: &Heap, o: Obj, c: u16) -> bool {
    let f = flags(heap, o);
    FLAG_CHARS
        .iter()
        .any(|(ch, bit)| *ch as u16 == c && f.contains(*bit))
}

/// EscapeRegExpPattern as QuickJS does it: `/` outside classes and line terminators
/// escaped, `(?:)` for the empty pattern.
pub(crate) fn escaped_source(heap: &mut Heap, o: Obj) -> Result<Str, Fault> {
    let src = source(heap, o);
    let units = heap.str_to_utf16(src);
    if units.is_empty() {
        return Ok(heap.alloc_str("(?:)")?);
    }
    let mut out = crate::buf::vec_with_capacity(units.len() + 4);
    let mut in_class = false;
    let mut i = 0;
    while i < units.len() {
        let c = units[i];
        i += 1;
        match c {
            0x5C => {
                out.push_out(c);
                if i < units.len() {
                    out.push_out(units[i]);
                    i += 1;
                }
            }
            0x5D => {
                in_class = false;
                out.push_out(c);
            }
            0x5B => {
                out.push_out(c);
                if !in_class {
                    in_class = true;
                    // `[]` and `[^]`-style leading `]` stays literal.
                    if i < units.len() && units[i] == 0x5D {
                        out.push_out(units[i]);
                        i += 1;
                    }
                }
            }
            0x0A => out.extend_from_slice(&[0x5C, b'n' as u16]),
            0x0D => out.extend_from_slice(&[0x5C, b'r' as u16]),
            0x2F if !in_class => out.extend_from_slice(&[0x5C, 0x2F]),
            _ => out.push_out(c),
        }
    }
    if out == units {
        return Ok(src);
    }
    Ok(alloc_units(heap, &out)?)
}

fn alloc_units(heap: &mut Heap, units: &[u16]) -> Result<Str, HeapError> {
    if units.iter().all(|&u| u <= 0xFF) {
        let bytes: Vec<u8> = units.iter().map(|&u| u as u8).collect();
        heap.alloc_latin1(&bytes)
    } else {
        heap.alloc_utf16(units)
    }
}

/// Sets `lastIndex` (an own data property that is never an accessor: it is
/// non-configurable), with Set's TypeError when it is read-only.
pub(crate) fn set_last_index(heap: &mut Heap, o: Obj, v: Value) -> Result<(), Fault> {
    let key = atoms::LAST_INDEX;
    match heap.get_own_property(o, key) {
        Some(Property::Data { flags, .. }) if !flags.writable() => {
            type_error("'lastIndex' is read-only")
        }
        _ => {
            let d = PropertyDescriptor {
                value: Some(v),
                ..Default::default()
            };
            heap.define_own_property(o, key, &d)?;
            Ok(())
        }
    }
}

/// The substring `s[start..end)` as a value.
pub(crate) fn substring_value(
    heap: &mut Heap,
    s: Str,
    start: usize,
    end: usize,
) -> Result<Value, Fault> {
    substring(heap, s, start, end)
}

/// The substring `s[start..end)` of a flat string view.
fn substring(heap: &mut Heap, s: Str, start: usize, end: usize) -> Result<Value, Fault> {
    Ok(Value::string(super::string::substring(
        heap,
        s,
        start as u32,
        end as u32,
    )?))
}

/// Defines `key: v` on a groups object, except that an undefined value never replaces
/// a defined one (duplicate named groups: only one of them participates).
fn define_group(heap: &mut Heap, groups: Obj, name: &[u32], v: Value) -> Result<(), Fault> {
    let units: Vec<u16> = name.iter().map(|&u| u as u16).collect();
    let key = heap.atom_utf16(&units)?;
    if v.is_undefined() && heap.has_own_property(groups, key) {
        return Ok(());
    }
    heap.create_data_property(groups, key, v)?;
    Ok(())
}

/// The match result array of RegExpBuiltinExec (22.2.7.2 steps 22-35): the captures,
/// `index`, `input`, `groups`, and with the `d` flag `indices`.
pub(crate) fn match_result(
    heap: &mut Heap,
    prog: &Program,
    s: Str,
    caps: &Captures,
) -> Result<Value, Fault> {
    let n = caps.len();
    let mut values = crate::buf::vec_with_capacity(n);
    for i in 0..n {
        values.push_out(match caps.get(i) {
            Some((a, b)) => substring(heap, s, a, b)?,
            None => Value::UNDEFINED,
        });
    }
    let array_proto = realm::intrinsic_obj(heap, Intrinsic::ArrayProto);
    let a = heap.new_array_from_values(Some(array_proto), &values)?;
    let (start, _) = caps.get(0).expect("group 0 matched");
    let index = atoms::INDEX;
    heap.create_data_property(a, index, Value::int(start as i32))?;
    let input = atoms::INPUT;
    heap.create_data_property(a, input, Value::string(s))?;
    let has_names = prog.has_group_names();
    let groups = if has_names {
        let g = heap.new_object(None)?;
        for (i, v) in values.iter().enumerate().skip(1) {
            if let Some(name) = prog.group_name(i) {
                define_group(heap, g, name, *v)?;
            }
        }
        Value::object(g)
    } else {
        Value::UNDEFINED
    };
    let groups_key = atoms::GROUPS;
    heap.create_data_property(a, groups_key, groups)?;
    if prog.flags().contains(Flags::HAS_INDICES) {
        let mut pairs = crate::buf::vec_with_capacity(n);
        for i in 0..n {
            pairs.push_out(match caps.get(i) {
                Some((a, b)) => {
                    let pair = [Value::int(a as i32), Value::int(b as i32)];
                    Value::object(heap.new_array_from_values(Some(array_proto), &pair)?)
                }
                None => Value::UNDEFINED,
            });
        }
        let indices = heap.new_array_from_values(Some(array_proto), &pairs)?;
        let igroups = if has_names {
            let g = heap.new_object(None)?;
            for (i, v) in pairs.iter().enumerate().skip(1) {
                if let Some(name) = prog.group_name(i) {
                    define_group(heap, g, name, *v)?;
                }
            }
            Value::object(g)
        } else {
            Value::UNDEFINED
        };
        heap.create_data_property(indices, groups_key, igroups)?;
        let key = heap.atom_str("indices")?;
        heap.create_data_property(a, key, Value::object(indices))?;
    }
    Ok(Value::object(a))
}

/// A new RegExp String Iterator.
pub(crate) fn string_iterator(heap: &mut Heap, r: Obj, s: Str, flags: i32) -> Result<Obj, Fault> {
    let proto = realm::intrinsic_obj(heap, Intrinsic::RegExpStringIteratorProto);
    let it = heap.alloc_object(ObjectClass::RegExpStringIterator, Some(proto), 3)?;
    heap.set_object_slot(it, IT_REGEXP, Value::object(r));
    heap.set_object_slot(it, IT_STRING, Value::string(s));
    heap.set_object_slot(it, IT_FLAGS, Value::int(flags));
    Ok(it)
}

/// The RegExp String Iterator `v`, or qjs's TypeError.
pub(crate) fn as_string_iterator(heap: &Heap, v: Value) -> Result<Obj, Fault> {
    match v
        .as_object()
        .filter(|&o| heap.object_class(o) == ObjectClass::RegExpStringIterator)
    {
        Some(o) => Ok(o),
        None => type_error("RegExp String Iterator object expected"),
    }
}

/// The input of the matcher: a flat string's code units, read in place.
pub(crate) fn input(view: StrView<'_>) -> regexp::Input<'_> {
    match view {
        StrView::Latin1(b) => regexp::Input::Latin1(b),
        StrView::Utf16(b) => regexp::Input::Utf16Le(b),
    }
}

/// RegExp.escape ( S ) of a string, as QuickJS writes it (lowercase hex).
pub(crate) fn escape(heap: &mut Heap, s: Str) -> Result<Str, Fault> {
    let units = heap.str_to_utf16(s);
    let mut out = String::new();
    let mut i = 0;
    while i < units.len() {
        let i0 = i;
        let mut c = units[i] as u32;
        i += 1;
        if (0xD800..0xDC00).contains(&c) && i < units.len() && (0xDC00..0xE000).contains(&units[i])
        {
            c = 0x10000 + ((c - 0xD800) << 10) + (units[i] as u32 - 0xDC00);
            i += 1;
        }
        let hex2 = c < 256
            && (c < 33 && !(9..=13).contains(&c)
                || c >= 128
                || (c as u8).is_ascii_alphanumeric() && i0 == 0
                || b",-=<>#&!%:;@~'`\"".contains(&(c as u8)));
        if hex2 {
            crate::text::cat_to!(&mut out, "\\x", crate::text::hex(c, 2));
        } else if (9..=13).contains(&c) {
            out.push_out('\\');
            out.push_out(b"tnvfr"[c as usize - 9] as char);
        } else if c < 128 {
            if !(c as u8).is_ascii_alphanumeric() && c != b'_' as u32 {
                out.push_out('\\');
            }
            out.push_out(c as u8 as char);
        } else if (0xD800..0xE000).contains(&c)
            || matches!(
                c,
                0x1680 | 0x2000..=0x200A | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000 | 0xFEFF
            )
        {
            crate::text::cat_to!(&mut out, "\\u", crate::text::hex(c, 4));
        } else {
            out.push_out(char::from_u32(c).unwrap_or('\u{FFFD}'));
        }
    }
    Ok(heap.alloc_str(&out)?)
}

/// AdvanceStringIndex ( S, index, unicode ).
pub(crate) fn advance(view: StrView<'_>, i: usize, unicode: bool) -> usize {
    if unicode
        && i + 1 < view.len()
        && (0xD800..0xDC00).contains(&view.get(i))
        && (0xDC00..0xE000).contains(&view.get(i + 1))
    {
        i + 2
    } else {
        i + 1
    }
}

/// GetSubstitution for a match of an unmodified RegExp (D1503): `$$`, `$&`, `` $` ``,
/// `$'`, `$n`/`$nn` and `$<name>` (only with named groups, else literal), appended to
/// `out`. `s` is the matched string.
pub(crate) fn substitute(
    out: &mut Vec<u16>,
    template: &[u16],
    s: StrView<'_>,
    caps: &Captures,
    prog: &Program,
) {
    let (m0, m1) = caps.get(0).expect("group 0 matched");
    let m = caps.len() - 1;
    let push_range = |out: &mut Vec<u16>, a: usize, b: usize| {
        out.extend((a..b).map(|i| s.get(i)));
    };
    let mut i = 0;
    while i < template.len() {
        let c = template[i];
        if c != b'$' as u16 || i + 1 >= template.len() {
            out.push_out(c);
            i += 1;
            continue;
        }
        let d = template[i + 1];
        match d {
            0x24 => {
                out.push_out(0x24);
                i += 2;
            }
            0x26 => {
                push_range(out, m0, m1);
                i += 2;
            }
            0x60 => {
                push_range(out, 0, m0);
                i += 2;
            }
            0x27 => {
                push_range(out, m1.min(s.len()), s.len());
                i += 2;
            }
            0x30..=0x39 => {
                let mut index = (d - 0x30) as usize;
                let mut digits = 1;
                if let Some(&e) = template.get(i + 2)
                    && (0x30..=0x39).contains(&e)
                {
                    let two = index * 10 + (e - 0x30) as usize;
                    if (1..=m).contains(&two) {
                        index = two;
                        digits = 2;
                    }
                }
                if (1..=m).contains(&index) {
                    if let Some((a, b)) = caps.get(index) {
                        push_range(out, a, b);
                    }
                    i += 1 + digits;
                } else {
                    out.push_out(0x24);
                    i += 1;
                }
            }
            0x3C if prog.has_group_names() => {
                match template[i + 2..].iter().position(|&u| u == b'>' as u16) {
                    None => {
                        out.extend_from_slice(&[0x24, 0x3C]);
                        i += 2;
                    }
                    Some(k) => {
                        let name = &template[i + 2..i + 2 + k];
                        // The participating group of that name, if any (duplicate names).
                        let span = (1..caps.len())
                            .filter(|&g| {
                                prog.group_name(g).is_some_and(|n| {
                                    n.len() == name.len()
                                        && n.iter().zip(name).all(|(&a, &b)| a == b as u32)
                                })
                            })
                            .find_map(|g| caps.get(g));
                        if let Some((a, b)) = span {
                            push_range(out, a, b);
                        }
                        i += 3 + k;
                    }
                }
            }
            _ => {
                out.push_out(0x24);
                i += 1;
            }
        }
    }
}

/// A heap string from UTF-16 units (one byte per unit when possible).
pub(crate) fn string_from_units(heap: &mut Heap, units: &[u16]) -> Result<Str, HeapError> {
    alloc_units(heap, units)
}
