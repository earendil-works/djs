//! Leaf fast paths of everyday builtins (D1850, 0085).
//!
//! A builtin tagged `@fast` in `builtins/*.ts` has an entry here under its function
//! name; the prelude marks it with `%MarkFast(f)`. With native builtins on, a call of it
//! first runs the entry with `this` and the arguments. The entry either returns the
//! builtin's result, or declines (`Ok(None)`) and the builtin runs as usual.
//!
//! The rule that keeps this unobservable (the D1351 rule): an entry handles only
//! arguments for which the builtin's algorithm cannot run user code or look anything up
//! that user code could have changed. In practice: string, number, boolean, undefined
//! and null arguments (ToString, ToNumber, ToIntegerOrInfinity and ToPropertyKey of those
//! are pure), and objects only where the algorithm reads them with a leaf operation (an
//! own-property check of a non-proxy). Anything else, and every case where the builtin
//! would throw, declines, so errors and their stack traces come from the builtin itself.
//! Each entry mirrors its TS source step by step; keep them in sync.

use alloc::vec::Vec;

use crate::builtins::{dense, number, object, string};
use crate::bytecode::intrinsics::INTRINSICS;
use crate::heap::string::MAX_LEN;
use crate::heap::{Heap, ObjectClass, Property, PropertyDescriptor, StrView, atoms};
use crate::interp::conv::{self, Fault};
use crate::interp::{Intrinsic, realm};
use crate::value::{Kind, Obj, Str, Value};

/// The result of an entry: `None` declines.
pub(crate) type Fast = Result<Option<Value>, Fault>;

pub(crate) struct FastFn {
    /// The builtin's function name in the prelude (checked by `%MarkFast`).
    pub name: &'static str,
    pub run: Run,
}

#[derive(Clone, Copy)]
pub(crate) enum Run {
    /// `f(heap, this, args)`.
    Fn(fn(&mut Heap, Value, &[Value]) -> Fast),
    /// A `Math` function `F64X(ToNumber(x))`.
    Math1(fn(f64) -> f64),
}

/// The most arguments an entry looks at (calls with more decline).
pub(crate) const MAX_ARGS: usize = 6;

impl FastFn {
    pub(crate) fn call(&self, heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
        match self.run {
            Run::Fn(f) => f(heap, this, args),
            Run::Math1(f) => Ok(num(heap, arg(args, 0))?.map(|x| conv::number(f(x)))),
        }
    }
}

/// The index of the entry for builtin `name`.
pub(crate) fn index(name: &str) -> Option<usize> {
    FAST.iter().position(|f| f.name == name)
}

const fn f(name: &'static str, run: fn(&mut Heap, Value, &[Value]) -> Fast) -> FastFn {
    FastFn {
        name,
        run: Run::Fn(run),
    }
}

const fn m(name: &'static str, run: fn(f64) -> f64) -> FastFn {
    FastFn {
        name,
        run: Run::Math1(run),
    }
}

pub(crate) static FAST: &[FastFn] = &[
    // ---- conversion helpers of the builtin language (calls from bytecode builtins) ----
    f("ToString", to_string),
    f("ToNumber", to_number),
    f("ToIntegerOrInfinity", to_integer_or_infinity),
    f("ToLength", to_length),
    f("ToPropertyKey", to_property_key),
    f("ThisToString", this_to_string),
    f("RequireThisCoercible", require_this_coercible),
    f("IsRegExp", is_regexp),
    f("ClampPosition", clamp_position),
    f("RelativeIndex", relative_index),
    f("StrLen", str_len),
    // ---- String ----
    f("StringConstructor", string_constructor),
    f("StringPrototypeIncludes", includes),
    f("StringPrototypeIndexOf", index_of),
    f("StringPrototypeLastIndexOf", last_index_of),
    f("StringPrototypeStartsWith", starts_with),
    f("StringPrototypeEndsWith", ends_with),
    f("StringPrototypeSlice", slice),
    f("StringPrototypeSubstring", substring),
    f("StringPrototypeCharAt", char_at),
    f("StringPrototypeCharCodeAt", char_code_at),
    f("StringPrototypeCodePointAt", code_point_at),
    f("StringPrototypeAt", at),
    f("StringPrototypeRepeat", repeat),
    f("StringPrototypePadEnd", pad_end),
    f("StringPrototypePadStart", pad_start),
    f("StringPrototypeToLowerCase", to_lower_case),
    f("StringPrototypeToUpperCase", to_upper_case),
    f("StringPrototypeToLocaleLowerCase", to_lower_case),
    f("StringPrototypeToLocaleUpperCase", to_upper_case),
    f("StringPrototypeTrim", trim),
    f("StringPrototypeTrimStart", trim_start),
    f("StringPrototypeTrimEnd", trim_end),
    f("StringPrototypeSplit", split),
    f("StringPrototypeReplace", replace),
    f("StringPrototypeReplaceAll", replace_all),
    // ---- Number ----
    f("NumberConstructor", number_constructor),
    f("GlobalParseInt", parse_int),
    f("GlobalParseFloat", parse_float),
    f("GlobalIsNaN", is_nan),
    f("GlobalIsFinite", is_finite),
    f("NumberPrototypeToString", number_to_string),
    f("NumberPrototypeToFixed", number_to_fixed),
    // ---- Math ----
    f("MathMax", math_max),
    f("MathMin", math_min),
    m("MathAbs", libm::fabs),
    m("MathCeil", libm::ceil),
    m("MathFloor", libm::floor),
    m("MathRound", number::round),
    m("MathSign", number::sign),
    m("MathSqrt", libm::sqrt),
    m("MathTrunc", libm::trunc),
    m("MathExp", libm::exp),
    m("MathLog", libm::log),
    m("MathLog2", libm::log2),
    m("MathLog10", libm::log10),
    m("MathSin", libm::sin),
    m("MathCos", libm::cos),
    // ---- Object / Array ----
    f("ObjectPrototypeHasOwnProperty", has_own_property),
    f("ObjectHasOwn", object_has_own),
    f("ArrayIsArray", array_is_array),
    f("ArraySpeciesCreate", array_species_create),
    // ---- Array element loops over dense arrays (0091, builtins/dense.rs) ----
    f("ArrayPrototypePush", array_push),
    f("ArrayPrototypePop", array_pop),
    f("ArrayPrototypeIndexOf", array_index_of),
    f("ArrayPrototypeLastIndexOf", array_last_index_of),
    f("ArrayPrototypeIncludes", array_includes),
    f("ArrayPrototypeSlice", array_slice),
    f("ArrayPrototypeJoin", array_join),
    f("ArrayPrototypeFill", array_fill),
    // ---- String.fromCharCode (0091; the URI functions run their builtins, D2256) ----
    f("StringFromCharCode", from_char_code),
];

// ---- argument helpers ----------------------------------------------------------------

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).copied().unwrap_or(Value::UNDEFINED)
}

/// ToNumber of a primitive that cannot throw (not a symbol or bigint); `None` otherwise.
fn num(heap: &mut Heap, v: Value) -> Result<Option<f64>, Fault> {
    if let Some(n) = v.as_number() {
        return Ok(Some(n));
    }
    match v.kind() {
        Kind::Undefined | Kind::Null | Kind::Bool | Kind::String => {
            Ok(Some(conv::to_number(heap, v)?))
        }
        _ => Ok(None),
    }
}

/// ToString of a primitive that cannot throw (not a symbol); `None` otherwise.
fn str(heap: &mut Heap, v: Value) -> Result<Option<Str>, Fault> {
    if let Some(s) = v.as_string() {
        return Ok(Some(s));
    }
    match v.kind() {
        Kind::Undefined | Kind::Null | Kind::Bool | Kind::Number | Kind::BigInt => {
            Ok(Some(conv::to_string(heap, v)?))
        }
        _ => Ok(None),
    }
}

/// ThisToString / RequireObjectCoercible + ToString of a primitive `this`.
fn this_str(heap: &mut Heap, v: Value) -> Result<Option<Str>, Fault> {
    if v.is_undefined() || v.is_null() {
        return Ok(None);
    }
    str(heap, v)
}

/// ToIntegerOrInfinity of a Number.
fn integer(n: f64) -> f64 {
    if n.is_nan() {
        0.0
    } else {
        libm::trunc(n) + 0.0
    }
}

/// `ToIntegerOrInfinity(v)` for a primitive `v`.
fn int_arg(heap: &mut Heap, v: Value) -> Result<Option<f64>, Fault> {
    Ok(num(heap, v)?.map(integer))
}

/// ClampPosition(v, len): ToIntegerOrInfinity, clamped to [0, len].
fn clamp(p: f64, len: f64) -> f64 {
    p.max(0.0).min(len)
}

/// RelativeIndex(v, len, dflt) of array.ts.
fn relative(heap: &mut Heap, v: Value, len: f64, dflt: f64) -> Result<Option<f64>, Fault> {
    if v.is_undefined() {
        return Ok(Some(dflt));
    }
    let Some(rel) = int_arg(heap, v)? else {
        return Ok(None);
    };
    Ok(Some(if rel < 0.0 {
        (len + rel).max(0.0)
    } else {
        rel.min(len)
    }))
}

fn len(heap: &Heap, s: Str) -> f64 {
    heap.str_len(s) as f64
}

fn some_str(s: Str) -> Fast {
    Ok(Some(Value::string(s)))
}

fn empty(heap: &mut Heap) -> Fast {
    some_str(heap.atom_string(atoms::EMPTY)?)
}

/// Whether `s[at .. at + len(t))` is `t` (the caller checked the bounds).
fn region_eq(heap: &mut Heap, s: Str, at: u32, t: Str) -> Result<bool, Fault> {
    let t = heap.flatten(t)?;
    let s = heap.flatten(s)?;
    let (Some(a), Some(b)) = (heap.str_view(s), heap.str_view(t)) else {
        return Ok(false);
    };
    let at = at as usize;
    Ok(match (a, b) {
        (StrView::Latin1(x), StrView::Latin1(y)) => &x[at..at + y.len()] == y,
        _ => (0..b.len()).all(|j| a.get(at + j) == b.get(j)),
    })
}

// ---- conversion helpers --------------------------------------------------------------

fn to_string(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    Ok(str(heap, arg(args, 0))?.map(Value::string))
}

fn to_number(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    Ok(num(heap, arg(args, 0))?.map(conv::number))
}

fn to_integer_or_infinity(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    Ok(int_arg(heap, arg(args, 0))?.map(conv::number))
}

fn to_length(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    Ok(int_arg(heap, arg(args, 0))?.map(|n| conv::number(n.clamp(0.0, 9007199254740991.0))))
}

fn to_property_key(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    let v = arg(args, 0);
    if v.as_symbol().is_some() || v.is_number() {
        return Ok(Some(v));
    }
    Ok(str(heap, v)?.map(Value::string))
}

fn this_to_string(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    Ok(this_str(heap, arg(args, 0))?.map(Value::string))
}

fn require_this_coercible(_: &mut Heap, _: Value, args: &[Value]) -> Fast {
    let v = arg(args, 0);
    Ok((!v.is_undefined() && !v.is_null()).then_some(Value::UNDEFINED))
}

fn is_regexp(_: &mut Heap, _: Value, args: &[Value]) -> Fast {
    // Only primitives: an object's @@match is a lookup.
    Ok(arg(args, 0).as_object().is_none().then_some(Value::FALSE))
}

fn clamp_position(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    let (Some(p), Some(len)) = (int_arg(heap, arg(args, 0))?, arg(args, 1).as_number()) else {
        return Ok(None);
    };
    Ok(Some(conv::number(clamp(p, len))))
}

fn relative_index(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    let (Some(len), Some(dflt)) = (arg(args, 1).as_number(), arg(args, 2).as_number()) else {
        return Ok(None);
    };
    Ok(relative(heap, arg(args, 0), len, dflt)?.map(conv::number))
}

fn str_len(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    Ok(arg(args, 0)
        .as_string()
        .map(|s| Value::int(heap.str_len(s) as i32)))
}

// ---- String --------------------------------------------------------------------------

/// `String(value)` called as a function (the call path never has a new.target).
fn string_constructor(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    if args.is_empty() {
        return empty(heap);
    }
    Ok(str(heap, args[0])?.map(Value::string))
}

/// The receiver and the search string of includes/indexOf/startsWith/...
fn search(heap: &mut Heap, this: Value, search: Value) -> Result<Option<(Str, Str)>, Fault> {
    let Some(s) = this_str(heap, this)? else {
        return Ok(None);
    };
    // A primitive is never a RegExp (IsRegExp looks up @@match only on objects).
    Ok(str(heap, search)?.map(|t| (s, t)))
}

fn includes(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some((s, t)) = search(heap, this, arg(args, 0))? else {
        return Ok(None);
    };
    let Some(p) = int_arg(heap, arg(args, 1))? else {
        return Ok(None);
    };
    let start = clamp(p, len(heap, s)) as u32;
    Ok(Some(Value::bool(
        string::index_of(heap, s, t, start, false)? >= 0.0,
    )))
}

fn index_of(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some((s, t)) = search(heap, this, arg(args, 0))? else {
        return Ok(None);
    };
    let Some(p) = int_arg(heap, arg(args, 1))? else {
        return Ok(None);
    };
    let start = clamp(p, len(heap, s)) as u32;
    Ok(Some(conv::number(string::index_of(
        heap, s, t, start, false,
    )?)))
}

fn last_index_of(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some((s, t)) = search(heap, this, arg(args, 0))? else {
        return Ok(None);
    };
    let Some(num_pos) = num(heap, arg(args, 1))? else {
        return Ok(None);
    };
    let n = len(heap, s);
    // NaN means +Infinity here.
    let pos = if num_pos.is_nan() {
        n
    } else {
        clamp(integer(num_pos), n)
    };
    let m = len(heap, t);
    if m > n {
        return Ok(Some(Value::int(-1)));
    }
    let from = pos.min(n - m) as u32;
    Ok(Some(conv::number(string::index_of(
        heap, s, t, from, true,
    )?)))
}

fn starts_with(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some((s, t)) = search(heap, this, arg(args, 0))? else {
        return Ok(None);
    };
    let Some(p) = int_arg(heap, arg(args, 1))? else {
        return Ok(None);
    };
    let n = len(heap, s);
    let start = clamp(p, n);
    if start + len(heap, t) > n {
        return Ok(Some(Value::FALSE));
    }
    Ok(Some(Value::bool(region_eq(heap, s, start as u32, t)?)))
}

fn ends_with(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some((s, t)) = search(heap, this, arg(args, 0))? else {
        return Ok(None);
    };
    let n = len(heap, s);
    let end = match arg(args, 1) {
        v if v.is_undefined() => n,
        v => match int_arg(heap, v)? {
            Some(p) => clamp(p, n),
            None => return Ok(None),
        },
    };
    let m = len(heap, t);
    if m > end {
        return Ok(Some(Value::FALSE));
    }
    Ok(Some(Value::bool(region_eq(heap, s, (end - m) as u32, t)?)))
}

fn slice(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some(s) = this_str(heap, this)? else {
        return Ok(None);
    };
    let n = len(heap, s);
    let (Some(from), Some(to)) = (
        relative(heap, arg(args, 0), n, 0.0)?,
        relative(heap, arg(args, 1), n, n)?,
    ) else {
        return Ok(None);
    };
    if from >= to {
        return empty(heap);
    }
    some_str(string::substring(heap, s, from as u32, to as u32)?)
}

fn substring(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some(s) = this_str(heap, this)? else {
        return Ok(None);
    };
    let n = len(heap, s);
    let Some(a) = int_arg(heap, arg(args, 0))? else {
        return Ok(None);
    };
    let b = match arg(args, 1) {
        v if v.is_undefined() => n,
        v => match int_arg(heap, v)? {
            Some(p) => p,
            None => return Ok(None),
        },
    };
    let (a, b) = (clamp(a, n), clamp(b, n));
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    some_str(string::substring(heap, s, lo as u32, hi as u32)?)
}

/// The code unit index of charAt/charCodeAt/codePointAt: `None` declines, `Some(None)`
/// is out of range.
fn position(heap: &mut Heap, s: Str, v: Value) -> Result<Option<Option<u32>>, Fault> {
    let Some(p) = int_arg(heap, v)? else {
        return Ok(None);
    };
    Ok(Some((p >= 0.0 && p < len(heap, s)).then_some(p as u32)))
}

fn char_at(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some(s) = this_str(heap, this)? else {
        return Ok(None);
    };
    match position(heap, s, arg(args, 0))? {
        None => Ok(None),
        Some(None) => empty(heap),
        Some(Some(i)) => some_str(string::substring(heap, s, i, i + 1)?),
    }
}

fn char_code_at(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some(s) = this_str(heap, this)? else {
        return Ok(None);
    };
    Ok(match position(heap, s, arg(args, 0))? {
        None => None,
        Some(None) => Some(Value::NAN),
        Some(Some(i)) => {
            let s = heap.flatten(s)?;
            Some(Value::int(heap.char_code_at(s, i).unwrap_or(0) as i32))
        }
    })
}

fn code_point_at(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some(s) = this_str(heap, this)? else {
        return Ok(None);
    };
    Ok(match position(heap, s, arg(args, 0))? {
        None => None,
        Some(None) => Some(Value::UNDEFINED),
        Some(Some(i)) => {
            let s = heap.flatten(s)?;
            Some(string::code_point_at(heap, s, i).map_or(Value::UNDEFINED, conv::number))
        }
    })
}

fn at(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some(s) = this_str(heap, this)? else {
        return Ok(None);
    };
    let Some(rel) = int_arg(heap, arg(args, 0))? else {
        return Ok(None);
    };
    let n = len(heap, s);
    let k = if rel >= 0.0 { rel } else { n + rel };
    if k < 0.0 || k >= n {
        return Ok(Some(Value::UNDEFINED));
    }
    some_str(string::substring(heap, s, k as u32, k as u32 + 1)?)
}

fn repeat(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some(s) = this_str(heap, this)? else {
        return Ok(None);
    };
    let Some(n) = int_arg(heap, arg(args, 0))? else {
        return Ok(None);
    };
    if n < 0.0 || n == f64::INFINITY {
        return Ok(None); // RangeError
    }
    if heap.str_len(s) == 0 || n == 0.0 {
        return empty(heap);
    }
    // Too long: decline, the builtin throws the RangeError.
    if n * len(heap, s) > MAX_LEN as f64 {
        return Ok(None);
    }
    some_str(string::repeat(heap, s, n as u32)?)
}

/// StringPad ( S, maxLength, fillString, placement ).
fn pad(heap: &mut Heap, this: Value, args: &[Value], at_start: bool) -> Fast {
    let Some(s) = this_str(heap, this)? else {
        return Ok(None);
    };
    let Some(max) = int_arg(heap, arg(args, 0))? else {
        return Ok(None);
    };
    let max = max.clamp(0.0, 9007199254740991.0);
    let n = len(heap, s);
    if max <= n {
        return some_str(s);
    }
    let fill = arg(args, 1);
    let filler = if fill.is_undefined() {
        heap.alloc_latin1(b" ")?
    } else {
        match str(heap, fill)? {
            Some(f) => f,
            None => return Ok(None),
        }
    };
    let flen = heap.str_len(filler);
    if flen == 0 {
        return some_str(s);
    }
    if max > MAX_LEN as f64 {
        return Ok(None); // RangeError from the builtin
    }
    // The filler repeated and truncated to `max - n` units, and `s`, in one string.
    let (n, fill_len) = (n as u32, (max - n) as u32);
    if !heap.str_is_two_byte(s) && !heap.str_is_two_byte(filler) && max as usize <= 256 {
        let mut buf = [0u8; 256];
        let total = max as usize;
        let (fill_at, s_at) = if at_start {
            (0, fill_len as usize)
        } else {
            (n as usize, 0)
        };
        let f = heap.str_view_flat(filler)?.bytes();
        for (i, b) in buf[fill_at..fill_at + fill_len as usize]
            .iter_mut()
            .enumerate()
        {
            *b = f[i % flen as usize];
        }
        let v = heap.str_view_flat(s)?.bytes();
        buf[s_at..s_at + n as usize].copy_from_slice(v);
        return some_str(heap.alloc_latin1(&buf[..total])?);
    }
    let mut pieces: Vec<(Str, u32, u32)> = Vec::with_capacity((fill_len / flen + 2) as usize);
    if !at_start {
        pieces.push((s, 0, n));
    }
    let mut left = fill_len;
    while left > 0 {
        let k = left.min(flen);
        pieces.push((filler, 0, k));
        left -= k;
    }
    if at_start {
        pieces.push((s, 0, n));
    }
    some_str(string::join(heap, &mut pieces)?)
}

fn pad_end(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    pad(heap, this, args, false)
}

fn pad_start(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    pad(heap, this, args, true)
}

fn to_lower_case(heap: &mut Heap, this: Value, _: &[Value]) -> Fast {
    match this_str(heap, this)? {
        Some(s) => some_str(string::to_case(heap, s, false)?),
        None => Ok(None),
    }
}

fn to_upper_case(heap: &mut Heap, this: Value, _: &[Value]) -> Fast {
    match this_str(heap, this)? {
        Some(s) => some_str(string::to_case(heap, s, true)?),
        None => Ok(None),
    }
}

fn trim_mode(heap: &mut Heap, this: Value, mode: i32) -> Fast {
    match this_str(heap, this)? {
        Some(s) => some_str(string::trim(heap, s, mode)?),
        None => Ok(None),
    }
}

fn trim(heap: &mut Heap, this: Value, _: &[Value]) -> Fast {
    trim_mode(heap, this, 0)
}

fn trim_start(heap: &mut Heap, this: Value, _: &[Value]) -> Fast {
    trim_mode(heap, this, 1)
}

fn trim_end(heap: &mut Heap, this: Value, _: &[Value]) -> Fast {
    trim_mode(heap, this, 2)
}

fn new_array(heap: &mut Heap, values: &[Value]) -> Fast {
    let proto = realm::intrinsic_obj(heap, Intrinsic::ArrayProto);
    Ok(Some(Value::object(
        heap.new_array_from_values(Some(proto), values)?,
    )))
}

/// String.prototype.split with a primitive separator (an object's @@split is a lookup).
fn split(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let (sep, limit) = (arg(args, 0), arg(args, 1));
    if sep.as_object().is_some() {
        return Ok(None);
    }
    let Some(s) = this_str(heap, this)? else {
        return Ok(None);
    };
    let lim = if limit.is_undefined() {
        u32::MAX
    } else {
        match num(heap, limit)? {
            Some(n) => number::to_uint32(n),
            None => return Ok(None),
        }
    };
    let Some(r) = str(heap, sep)? else {
        return Ok(None);
    };
    if lim == 0 {
        return new_array(heap, &[]);
    }
    if sep.is_undefined() {
        return new_array(heap, &[Value::string(s)]);
    }
    let n = heap.str_len(s);
    let m = heap.str_len(r);
    let mut parts = Vec::new();
    if m == 0 {
        for i in 0..n.min(lim) {
            parts.push(Value::string(string::substring(heap, s, i, i + 1)?));
        }
        return new_array(heap, &parts);
    }
    // One-byte string and separator: find every piece in one pass over the bytes, then
    // copy them (0110: a search per piece repeated the setup of `index_of`).
    heap.str_view_flat(s)?;
    let r = heap.flatten(r)?;
    if let (Some(StrView::Latin1(h)), Some(StrView::Latin1(t))) =
        (heap.str_view(s), heap.str_view(r))
    {
        let mut cuts: Vec<(u32, u32)> = Vec::with_capacity(16);
        let mut p = 0;
        while let Some(q) = string::find_latin1(h, t, p) {
            cuts.push((p as u32, q as u32));
            if cuts.len() as u32 == lim {
                break;
            }
            p = q + t.len();
        }
        if (cuts.len() as u32) < lim {
            cuts.push((p as u32, n));
        }
        // Each piece copied straight from `s` (flat, one-byte), the empty ones shared
        // (0129: through `substring` and `join` each paid their checks again).
        let empty = Value::string(heap.atom_string(atoms::EMPTY)?);
        parts.reserve_exact(cuts.len());
        for (a, b) in cuts {
            parts.push(if a == b {
                empty
            } else if a == 0 && b == n {
                Value::string(s)
            } else {
                Value::string(heap.alloc_latin1_pieces(&[(s, a, b)], b - a)?)
            });
        }
        return new_array(heap, &parts);
    }
    let mut p = 0;
    let mut q = string::index_of(heap, s, r, 0, false)?;
    while q >= 0.0 {
        parts.push(Value::string(string::substring(heap, s, p, q as u32)?));
        if parts.len() as u32 == lim {
            return new_array(heap, &parts);
        }
        p = q as u32 + m;
        q = string::index_of(heap, s, r, p, false)?;
    }
    parts.push(Value::string(string::substring(heap, s, p, n)?));
    new_array(heap, &parts)
}

/// The string and the search and replacement strings of replace/replaceAll; `None`
/// for an object or callable search value or replacement, and for a replacement with
/// `$` (GetSubstitution patterns stay in the builtin).
fn replace_args(
    heap: &mut Heap,
    this: Value,
    args: &[Value],
) -> Result<Option<(Str, Str, Str)>, Fault> {
    let (search, replacement) = (arg(args, 0), arg(args, 1));
    if search.as_object().is_some() || replacement.as_object().is_some() {
        return Ok(None);
    }
    let Some(s) = this_str(heap, this)? else {
        return Ok(None);
    };
    let (Some(t), Some(r)) = (str(heap, search)?, str(heap, replacement)?) else {
        return Ok(None);
    };
    if heap.str_units(r).any(|u| u == u16::from(b'$')) {
        return Ok(None);
    }
    Ok(Some((s, t, r)))
}

fn replace(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some((s, t, r)) = replace_args(heap, this, args)? else {
        return Ok(None);
    };
    let pos = string::index_of(heap, s, t, 0, false)?;
    if pos < 0.0 {
        return some_str(s);
    }
    let pos = pos as u32;
    let (n, m, rl) = (heap.str_len(s), heap.str_len(t), heap.str_len(r));
    some_str(string::join(
        heap,
        &mut [(s, 0, pos), (r, 0, rl), (s, pos + m, n)],
    )?)
}

fn replace_all(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some((s, t, r)) = replace_args(heap, this, args)? else {
        return Ok(None);
    };
    let (n, m, rl) = (heap.str_len(s), heap.str_len(t), heap.str_len(r));
    let advance = m.max(1);
    let mut pieces = Vec::new();
    let mut end_of_last = 0;
    let mut p = string::index_of(heap, s, t, 0, false)?;
    while p >= 0.0 {
        let q = p as u32;
        pieces.push((s, end_of_last, q));
        pieces.push((r, 0, rl));
        end_of_last = q + m;
        p = string::index_of(heap, s, t, q + advance, false)?;
    }
    if end_of_last < n {
        pieces.push((s, end_of_last, n));
    }
    some_str(string::join(heap, &mut pieces)?)
}

// ---- Number --------------------------------------------------------------------------

/// `Number(value)` called as a function: ToNumeric of a primitive (a BigInt converts).
fn number_constructor(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    if args.is_empty() {
        return Ok(Some(Value::int(0)));
    }
    Ok(num(heap, args[0])?.map(conv::number))
}

fn parse_int(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    let (Some(s), Some(r)) = (str(heap, arg(args, 0))?, num(heap, arg(args, 1))?) else {
        return Ok(None);
    };
    Ok(Some(conv::number(number::parse(
        heap,
        s,
        Some(conv::to_int32(r)),
    )?)))
}

fn parse_float(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    let Some(s) = str(heap, arg(args, 0))? else {
        return Ok(None);
    };
    Ok(Some(conv::number(number::parse(heap, s, None)?)))
}

fn is_nan(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    Ok(num(heap, arg(args, 0))?.map(|n| Value::bool(n.is_nan())))
}

fn is_finite(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    Ok(num(heap, arg(args, 0))?.map(|n| Value::bool(n.is_finite())))
}

fn number_to_string(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    // A Number primitive only (a wrapper's [[NumberData]] is fine too, but rare).
    let Some(x) = this.as_number() else {
        return Ok(None);
    };
    let radix = arg(args, 0);
    if radix.is_undefined() || radix.as_int() == Some(10) {
        return some_str(conv::number_to_string(heap, this)?);
    }
    let Some(r) = int_arg(heap, radix)? else {
        return Ok(None);
    };
    if !(2.0..=36.0).contains(&r) {
        return Ok(None); // RangeError
    }
    some_str(number::format(heap, 0, x, conv::number(r))?)
}

fn number_to_fixed(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some(x) = this.as_number() else {
        return Ok(None);
    };
    let Some(d) = int_arg(heap, arg(args, 0))? else {
        return Ok(None);
    };
    if !(0.0..=100.0).contains(&d) {
        return Ok(None); // RangeError
    }
    some_str(number::format(heap, 1, x, conv::number(d))?)
}

// ---- Math ----------------------------------------------------------------------------

fn math_max(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    let mut highest = f64::NEG_INFINITY;
    let mut nan = false;
    for &v in args {
        let Some(x) = num(heap, v)? else {
            return Ok(None);
        };
        nan |= x.is_nan();
        // +0 is larger than -0.
        if x > highest || (x == 0.0 && highest == 0.0 && x.is_sign_positive()) {
            highest = x;
        }
    }
    Ok(Some(if nan {
        Value::NAN
    } else {
        conv::number(highest)
    }))
}

fn math_min(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    let mut lowest = f64::INFINITY;
    let mut nan = false;
    for &v in args {
        let Some(x) = num(heap, v)? else {
            return Ok(None);
        };
        nan |= x.is_nan();
        // -0 is smaller than +0.
        if x < lowest || (x == 0.0 && lowest == 0.0 && x.is_sign_negative()) {
            lowest = x;
        }
    }
    Ok(Some(if nan {
        Value::NAN
    } else {
        conv::number(lowest)
    }))
}

// ---- Object / Array ------------------------------------------------------------------

/// HasOwnProperty(o, ToPropertyKey(key)) of a non-proxy object and a primitive key.
fn has_own(heap: &mut Heap, o: Value, key: Value) -> Fast {
    let Some(o) = o.as_object() else {
        return Ok(None);
    };
    if key.as_object().is_some() || heap.is_proxy(o) {
        return Ok(None);
    }
    let key = object::key(heap, key)?;
    Ok(Some(Value::bool(heap.has_own_property(o, key))))
}

fn has_own_property(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    has_own(heap, this, arg(args, 0))
}

fn object_has_own(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    has_own(heap, arg(args, 0), arg(args, 1))
}

fn array_is_array(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    Ok(match arg(args, 0).as_object() {
        None => Some(Value::FALSE),
        // A proxy answers for its target, and a revoked one throws: the builtin.
        Some(o) if heap.is_proxy(o) => None,
        Some(o) => heap.is_array(o).map(Value::bool),
    })
}

/// The id of an `@intrinsic` builtin, at compile time.
const fn intrinsic_id(name: &str) -> u16 {
    let mut k = 0;
    while k < INTRINSICS.len() {
        let (a, b) = (INTRINSICS[k].name.as_bytes(), name.as_bytes());
        if a.len() == b.len() {
            let mut j = 0;
            while j < a.len() && a[j] == b[j] {
                j += 1;
            }
            if j == a.len() {
                return k as u16;
            }
        }
        k += 1;
    }
    panic!("unknown intrinsic")
}

const ARRAY_CONSTRUCTOR: u16 = intrinsic_id("ArrayConstructor");
const ARRAY_SPECIES: u16 = intrinsic_id("ArraySpecies");

/// ArraySpeciesCreate(original, length) when nothing in it can run user code: `original`
/// is not an Array (and not a proxy), or an Array without an own `constructor` whose
/// prototype is %Array.prototype%, whose `constructor` is the data property %Array%,
/// whose @@species is the original getter (which returns %Array%). Then the result is
/// ArrayCreate(length), as Construct(%Array%, length) would make it. Anything else
/// (subclasses, patched constructors or species, proxies, a bad length) declines.
fn array_species_create(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    let (Some(o), Some(n)) = (arg(args, 0).as_object(), arg(args, 1).as_number()) else {
        return Ok(None);
    };
    if !(n >= 0.0 && n <= u32::MAX as f64 && (n as u32) as f64 == n) || !plain_species(heap, o) {
        return Ok(None);
    }
    let array_proto = realm::intrinsic_obj(heap, Intrinsic::ArrayProto);
    let a = heap.new_array(Some(array_proto), 0)?;
    let d = PropertyDescriptor {
        value: Some(conv::number(n)),
        ..Default::default()
    };
    heap.define_own_property(a, atoms::LENGTH, &d)?;
    Ok(Some(Value::object(a)))
}

/// Whether ArraySpeciesCreate(o, n) is ArrayCreate(n) with %Array.prototype%, found
/// without running user code: `o` is no Array, or an Array whose `constructor` is the
/// untouched %Array% with the original @@species getter.
fn plain_species(heap: &Heap, o: Obj) -> bool {
    if heap.is_proxy(o) {
        return false;
    }
    if heap.object_class(o) != ObjectClass::Array {
        return true;
    }
    let array_proto = realm::intrinsic_obj(heap, Intrinsic::ArrayProto);
    if heap.has_own_property(o, atoms::CONSTRUCTOR) || heap.get_prototype_of(o) != Some(array_proto)
    {
        return false;
    }
    let array = realm::installed_intrinsic(heap, ARRAY_CONSTRUCTOR);
    match heap.get_own_property(array_proto, atoms::CONSTRUCTOR) {
        Some(Property::Data { value, .. }) if value == array => {}
        _ => return false,
    }
    let Some(array) = array.as_object() else {
        return false;
    };
    let species = realm::installed_intrinsic(heap, ARRAY_SPECIES);
    matches!(heap.get_own_property(array, atoms::SYM_SPECIES),
        Some(Property::Accessor { getter, .. }) if getter == species)
}

// ---- Array element loops (0091) ------------------------------------------------------
// `this` must be an Array (ToObject is the identity, LengthOfArrayLike its length);
// the loops are builtins/dense.rs's, which decline at a hole, a non-dense element or
// anything else that could run user code.

/// An Array `this` and its length.
fn this_array(heap: &Heap, this: Value) -> Option<(Obj, u32)> {
    let o = this.as_object()?;
    (heap.object_class(o) == ObjectClass::Array).then(|| (o, heap.array_length(o)))
}

/// Array.prototype.push ( item ): one item as a plain append.
fn array_push(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let (Some((o, _)), [v]) = (this_array(heap, this), args) else {
        return Ok(None);
    };
    Ok(dense::push(heap, o, *v)?.map(|n| conv::number(n as f64)))
}

/// Array.prototype.pop ( ) of an array `Heap::array_pop_dense` takes.
fn array_pop(heap: &mut Heap, this: Value, _: &[Value]) -> Fast {
    let Some((o, _)) = this_array(heap, this) else {
        return Ok(None);
    };
    Ok(heap.array_pop_dense(o))
}

/// A search result of `dense::search` as the builtin's Number, `None` to decline.
fn found(r: f64) -> Option<f64> {
    (r != dense::DECLINE).then_some(r)
}

/// Array.prototype.indexOf ( searchElement [ , fromIndex ] ).
fn array_index_of(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some((o, len)) = this_array(heap, this) else {
        return Ok(None);
    };
    if len == 0 {
        return Ok(Some(Value::int(-1)));
    }
    let Some(mut n) = int_arg(heap, arg(args, 1))? else {
        return Ok(None);
    };
    let len_f = len as f64;
    if n >= len_f {
        return Ok(Some(Value::int(-1)));
    }
    if n < 0.0 {
        n = (len_f + n).max(0.0);
    }
    let (r, _) = dense::search(heap, o, arg(args, 0), n as u32, len, 0);
    Ok(found(r).map(conv::number))
}

/// Array.prototype.includes ( searchElement [ , fromIndex ] ).
fn array_includes(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some((o, len)) = this_array(heap, this) else {
        return Ok(None);
    };
    if len == 0 {
        return Ok(Some(Value::FALSE));
    }
    let Some(mut n) = int_arg(heap, arg(args, 1))? else {
        return Ok(None);
    };
    let len_f = len as f64;
    if n >= len_f {
        return Ok(Some(Value::FALSE));
    }
    if n < 0.0 {
        n = (len_f + n).max(0.0);
    }
    let (r, _) = dense::search(heap, o, arg(args, 0), n as u32, len, 2);
    Ok(found(r).map(|r| Value::bool(r >= 0.0)))
}

/// Array.prototype.lastIndexOf ( searchElement [ , fromIndex ] ).
fn array_last_index_of(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some((o, len)) = this_array(heap, this) else {
        return Ok(None);
    };
    if len == 0 {
        return Ok(Some(Value::int(-1)));
    }
    let len_f = len as f64;
    let n = if args.len() > 1 {
        match int_arg(heap, args[1])? {
            Some(n) => n,
            None => return Ok(None),
        }
    } else {
        len_f - 1.0
    };
    let k = if n >= 0.0 {
        n.min(len_f - 1.0)
    } else {
        len_f + n
    };
    if k < 0.0 {
        return Ok(Some(Value::int(-1)));
    }
    let (r, _) = dense::search(heap, o, arg(args, 0), k as u32, len, 1);
    Ok(found(r).map(conv::number))
}

/// Array.prototype.slice ( start, end ) with an Array result (ArraySpeciesCreate's
/// entry) that takes plain appends.
fn array_slice(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some((o, len)) = this_array(heap, this) else {
        return Ok(None);
    };
    let len_f = len as f64;
    let (Some(k), Some(fin)) = (
        relative(heap, arg(args, 0), len_f, 0.0)?,
        relative(heap, arg(args, 1), len_f, len_f)?,
    ) else {
        return Ok(None);
    };
    // A plain result, copied from dense elements in one step (0129). Anything else
    // declines: the builtin's own path (ArraySpeciesCreate, then `%ArraySliceDense`)
    // is what this did after it.
    if !plain_species(heap, o) {
        return Ok(None);
    }
    let proto = realm::intrinsic_obj(heap, Intrinsic::ArrayProto);
    let (k, fin) = (k as u32, fin.max(k) as u32);
    Ok(heap
        .new_array_from_dense(Some(proto), o, k, fin)?
        .map(Value::object))
}

/// Array.prototype.join ( separator ) with a primitive separator.
fn array_join(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some((o, len)) = this_array(heap, this) else {
        return Ok(None);
    };
    let sep = match arg(args, 0) {
        v if v.is_undefined() => heap.alloc_latin1(b",")?,
        v => match str(heap, v)? {
            Some(s) => s,
            None => return Ok(None),
        },
    };
    Ok(dense::join(heap, o, len, sep)?.map(Value::string))
}

/// Array.prototype.fill ( value [ , start [ , end ] ] ) while each Set is a plain
/// store. A partial fill declines: the builtin's loop repeats the same stores.
fn array_fill(heap: &mut Heap, this: Value, args: &[Value]) -> Fast {
    let Some((o, len)) = this_array(heap, this) else {
        return Ok(None);
    };
    let len_f = len as f64;
    let (Some(k), Some(fin)) = (
        relative(heap, arg(args, 1), len_f, 0.0)?,
        relative(heap, arg(args, 2), len_f, len_f)?,
    ) else {
        return Ok(None);
    };
    let (k, fin) = (k as u32, fin as u32);
    if fin > k && dense::fill(heap, o, arg(args, 0), k, fin)? < fin {
        return Ok(None);
    }
    Ok(Some(this))
}

// ---- String.fromCharCode (0091) -----------------------------------------------------

/// String.fromCharCode ( ...codeUnits ) of primitives (ToUint16 of ToNumber).
fn from_char_code(heap: &mut Heap, _: Value, args: &[Value]) -> Fast {
    let mut units: Vec<u16> = Vec::with_capacity(args.len());
    for &v in args {
        let Some(n) = num(heap, v)? else {
            return Ok(None);
        };
        units.push(conv::to_int32(n) as u16);
    }
    let s = if units.iter().all(|&u| u < 256) {
        let bytes: Vec<u8> = units.iter().map(|&u| u as u8).collect();
        heap.alloc_latin1(&bytes)?
    } else {
        heap.alloc_utf16(&units)?
    };
    some_str(s)
}
