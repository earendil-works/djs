//! Decoder tests: a test-only Rust encoder for building buffers, a canonical dump of heap
//! values (shared with the cross-language test), validation errors, and fuzzing.

extern crate std;

use super::*;
use crate::heap::{Heap, ObjectClass};
use crate::num;
use crate::value::Kind;
use std::format;
use std::string::{String, ToString};
use std::vec;
use std::vec::Vec;

/// A test value, in the shapes the format can carry.
#[derive(Clone, Debug)]
pub(super) enum T {
    Undef,
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Str16(Vec<u16>),
    Arr(Vec<Option<T>>),
    Obj(Vec<(String, T)>),
    Bytes(Vec<u8>),
}

/// The reference encoder (what js/src/wire.ts writes, without its padding tricks).
pub(super) struct Enc {
    pub(super) out: Vec<u8>,
    keys: Vec<String>,
}

impl Enc {
    pub(super) fn new() -> Enc {
        Enc {
            out: vec![b'D', b'J', b'W', VERSION],
            keys: Vec::new(),
        }
    }

    fn varint(&mut self, mut v: u32) {
        while v >= 0x80 {
            self.out.push((v as u8 & 0x7F) | 0x80);
            v >>= 7;
        }
        self.out.push(v as u8);
    }

    fn string(&mut self, s: &str) {
        self.out.push(tag::STR_UTF8);
        self.varint(s.encode_utf16().count() as u32);
        self.varint(s.len() as u32);
        self.out.extend_from_slice(s.as_bytes());
    }

    pub(super) fn value(&mut self, v: &T) {
        match v {
            T::Undef => self.out.push(tag::UNDEFINED),
            T::Null => self.out.push(tag::NULL),
            T::Bool(b) => self.out.push(if *b { tag::TRUE } else { tag::FALSE }),
            T::Num(n) => {
                let i = *n as i32;
                if i as f64 == *n && !(i == 0 && n.is_sign_negative()) {
                    if (-128..=127).contains(&i) {
                        self.out.push(tag::INT8);
                        self.out.push(i as i8 as u8);
                    } else {
                        self.out.push(tag::INT32);
                        self.out.extend_from_slice(&i.to_le_bytes());
                    }
                } else {
                    self.out.push(tag::FLOAT64);
                    self.out.extend_from_slice(&n.to_le_bytes());
                }
            }
            T::Str(s) => self.string(s),
            T::Str16(u) => {
                self.out.push(tag::STR_UTF16);
                self.varint(u.len() as u32);
                for &x in u {
                    self.out.extend_from_slice(&x.to_le_bytes());
                }
            }
            T::Arr(items) => {
                self.out.push(tag::ARRAY);
                self.varint(items.len() as u32);
                for it in items {
                    match it {
                        Some(v) => self.value(v),
                        None => self.out.push(tag::HOLE),
                    }
                }
            }
            T::Obj(props) => {
                self.out.push(tag::OBJECT);
                self.varint(props.len() as u32);
                for (k, v) in props {
                    match self.keys.iter().position(|x| x == k) {
                        Some(i) => self.varint(i as u32 + 1),
                        None => {
                            self.keys.push(k.clone());
                            self.out.push(0);
                            self.string(k);
                        }
                    }
                    self.value(v);
                }
            }
            T::Bytes(b) => {
                self.out.push(tag::BYTES);
                self.varint(b.len() as u32);
                self.out.extend_from_slice(b);
            }
        }
    }
}

pub(super) fn encode(v: &T) -> Vec<u8> {
    let mut e = Enc::new();
    e.value(v);
    e.out
}

fn quote(units: &[u16]) -> String {
    let mut s = String::from("\"");
    for &u in units {
        if (0x20..0x7F).contains(&u) && u != b'"' as u16 && u != b'\\' as u16 {
            s.push(u as u8 as char);
        } else {
            s.push_str(&format!("\\u{u:04x}"));
        }
    }
    s.push('"');
    s
}

/// A canonical dump: `undefined`, `null`, `true`, `n:<String(x)>`, `s:"..."` (non-printable
/// ASCII as `\uXXXX`), `[a,<hole>]`, `{"k":v}`. js/test/gen-cross.ts writes the same.
pub(super) fn dump(h: &mut Heap, v: Value) -> String {
    match v.kind() {
        Kind::Undefined => "undefined".into(),
        Kind::Null => "null".into(),
        Kind::Bool => format!("{}", v.as_bool().unwrap()),
        Kind::Number => {
            let n = v.as_number().unwrap();
            if n == 0.0 && n.is_sign_negative() {
                "n:-0".into()
            } else {
                format!("n:{}", num::to_string(n).as_str())
            }
        }
        Kind::String => format!("s:{}", quote(&h.str_to_utf16(v.as_string().unwrap()))),
        Kind::Object => {
            let o = v.as_object().unwrap();
            if h.object_class(o) == ObjectClass::Array {
                let mut items: Vec<String> = Vec::new();
                for i in 0..h.array_length(o) {
                    items.push(match h.get_own_property(o, Atom::int(i).unwrap()) {
                        Some(p) => dump(h, p.value().unwrap()),
                        None => "<hole>".into(),
                    });
                }
                format!("[{}]", items.join(","))
            } else {
                let mut items: Vec<String> = Vec::new();
                for k in h.own_keys(o) {
                    // The key's UTF-16 (atom_name would be lossy for lone surrogates).
                    let name_str = h.atom_string(k).unwrap();
                    let name = h.str_to_utf16(name_str);
                    let p = h.get_own_property(o, k).unwrap();
                    items.push(format!("{}:{}", quote(&name), dump(h, p.value().unwrap())));
                }
                format!("{{{}}}", items.join(","))
            }
        }
        k => format!("<{k:?}>"),
    }
}

fn dump_t(v: &T) -> String {
    match v {
        T::Undef => "undefined".into(),
        T::Null => "null".into(),
        T::Bool(b) => b.to_string(),
        T::Num(n) => {
            if *n == 0.0 && n.is_sign_negative() {
                "n:-0".into()
            } else {
                format!("n:{}", num::to_string(*n).as_str())
            }
        }
        T::Str(s) => format!("s:{}", quote(&s.encode_utf16().collect::<Vec<_>>())),
        T::Str16(u) => format!("s:{}", quote(u)),
        T::Arr(items) => {
            let parts: Vec<String> = items
                .iter()
                .map(|i| i.as_ref().map_or("<hole>".into(), dump_t))
                .collect();
            format!("[{}]", parts.join(","))
        }
        T::Obj(props) => {
            // JS property order: integer keys ascending, then the rest in order.
            let mut ints: Vec<(u32, &String, &T)> = Vec::new();
            let mut rest = Vec::new();
            for (k, v) in props {
                match k.parse::<u32>() {
                    Ok(i) if i.to_string() == *k && i < u32::MAX => ints.push((i, k, v)),
                    _ => rest.push((k, v)),
                }
            }
            ints.sort_by_key(|x| x.0);
            let parts: Vec<String> = ints
                .iter()
                .map(|(_, k, v)| (*k, *v))
                .chain(rest)
                .map(|(k, v)| {
                    format!(
                        "{}:{}",
                        quote(&k.encode_utf16().collect::<Vec<_>>()),
                        dump_t(v)
                    )
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        T::Bytes(b) => {
            let parts: Vec<String> = b.iter().map(|x| format!("n:{x}")).collect();
            format!("[{}]", parts.join(","))
        }
    }
}

fn decode_ok(bytes: &[u8]) -> (Heap, Value) {
    let mut h = Heap::new().unwrap();
    let v = decode(&mut h, bytes, &DecodeOptions::default()).unwrap_or_else(|e| panic!("{e}"));
    h.verify().unwrap();
    (h, v)
}

fn check(v: T) {
    let bytes = encode(&v);
    let (mut h, out) = decode_ok(&bytes);
    assert_eq!(dump(&mut h, out), dump_t(&v), "{v:?}");
}

fn s(x: &str) -> T {
    T::Str(x.into())
}

fn invalid(bytes: &[u8]) -> (usize, &'static str) {
    let mut h = Heap::new().unwrap();
    match decode(&mut h, bytes, &DecodeOptions::default()) {
        Err(WireError::Invalid { offset, reason }) => (offset, reason),
        other => panic!("expected an error, got {other:?}"),
    }
}

#[test]
fn primitives() {
    for v in [
        T::Undef,
        T::Null,
        T::Bool(true),
        T::Bool(false),
        T::Num(0.0),
        T::Num(-0.0),
        T::Num(1.0),
        T::Num(-128.0),
        T::Num(300.0),
        T::Num(-(2f64.powi(31))),
        T::Num(2f64.powi(31)),
        T::Num(1.5),
        T::Num(f64::NAN),
        T::Num(f64::INFINITY),
        T::Num(f64::MIN_POSITIVE),
    ] {
        check(v);
    }
    // A float64 that is integral decodes to the int32 form, like JSON.parse.
    let mut b = vec![b'D', b'J', b'W', 1, tag::FLOAT64];
    b.extend_from_slice(&7.0f64.to_le_bytes());
    let (_, v) = decode_ok(&b);
    assert_eq!(v.as_int(), Some(7));
}

#[test]
fn strings_choose_their_width() {
    for (text, two_byte) in [
        ("", false),
        ("ascii", false),
        ("caf\u{e9} \u{ff}", false),
        ("\u{100}", true),
        ("日本語", true),
        ("😀 smile", true),
        ("a\u{0}b", false),
    ] {
        let (h, v) = decode_ok(&encode(&s(text)));
        let st = v.as_string().unwrap();
        assert_eq!(h.str_to_utf16(st), text.encode_utf16().collect::<Vec<_>>());
        assert_eq!(h.str_is_two_byte(st), two_byte, "{text}");
    }
    // The UTF-16 form, with lone surrogates; narrowed when it can be.
    let (h, v) = decode_ok(&encode(&T::Str16(vec![0x61, 0xD800, 0x62])));
    assert_eq!(h.str_to_utf16(v.as_string().unwrap()), [0x61, 0xD800, 0x62]);
    let (h, v) = decode_ok(&encode(&T::Str16(vec![0x61, 0xE9])));
    assert!(!h.str_is_two_byte(v.as_string().unwrap()));
    // A padded byte length varint is accepted.
    let b = [
        b'D',
        b'J',
        b'W',
        1,
        tag::STR_UTF8,
        2,
        0x82,
        0x80,
        0x00,
        b'h',
        b'i',
    ];
    let (h, v) = decode_ok(&b);
    assert_eq!(h.str_to_string(v.as_string().unwrap()), "hi");
    check(s(&"x".repeat(100_000)));
    check(s(&"é".repeat(70_000)));
}

#[test]
fn containers() {
    check(T::Arr(vec![]));
    check(T::Obj(vec![]));
    check(T::Arr(vec![Some(T::Num(1.0)), None, Some(s("x")), None]));
    check(T::Obj(vec![
        ("a".into(), T::Num(1.0)),
        (
            "b".into(),
            T::Obj(vec![("a".into(), T::Arr(vec![Some(T::Null)]))]),
        ),
        ("".into(), T::Undef),
        ("10".into(), s("ten")),
        ("2".into(), s("two")),
        ("日本".into(), T::Bool(true)),
    ]));
    let rows: Vec<Option<T>> = (0..1000)
        .map(|i| {
            Some(T::Obj(vec![
                ("id".into(), T::Num(i as f64)),
                ("name".into(), s(&format!("row {i}"))),
                ("ok".into(), T::Bool(i % 2 == 0)),
            ]))
        })
        .collect();
    check(T::Arr(rows));
    check(T::Bytes(vec![0, 1, 2, 255]));
}

#[test]
fn depth_limit() {
    // Built and checked without recursion (the test helpers recurse).
    let nested = |n: u32| {
        let mut b = vec![b'D', b'J', b'W', 1];
        for _ in 0..n {
            b.extend_from_slice(&[tag::ARRAY, 1]);
        }
        b.extend_from_slice(&[tag::INT8, 7]);
        b
    };
    let (h, mut v) = decode_ok(&nested(DEFAULT_MAX_DEPTH));
    for _ in 0..DEFAULT_MAX_DEPTH {
        let o = v.as_object().unwrap();
        v = h
            .get_own_property(o, Atom::int(0).unwrap())
            .unwrap()
            .value()
            .unwrap();
    }
    assert_eq!(v.as_int(), Some(7));
    assert_eq!(
        invalid(&nested(DEFAULT_MAX_DEPTH + 1)).1,
        "nesting too deep"
    );
}

#[test]
fn keys_are_interned_once_and_repeated_keys_follow_js() {
    let b = encode(&T::Arr(vec![
        Some(T::Obj(vec![("k".into(), T::Num(1.0))])),
        Some(T::Obj(vec![("k".into(), T::Num(2.0))])),
    ]));
    // The second object refers to the key table: [.., OBJECT, 1, 1, INT8, 2].
    assert_eq!(&b[b.len() - 5..], &[tag::OBJECT, 1, 1, tag::INT8, 2]);
    check(T::Obj(vec![
        ("0".into(), T::Num(0.0)),
        ("x".into(), T::Num(1.0)),
    ]));
    // A repeated key: first position, last value.
    let mut e = Enc::new();
    e.out.extend_from_slice(&[tag::OBJECT, 3, 0]);
    e.string("a");
    e.out.extend_from_slice(&[tag::INT8, 1, 0]);
    e.string("b");
    e.out.extend_from_slice(&[tag::INT8, 2, 1, tag::INT8, 3]);
    let (mut h, v) = decode_ok(&e.out);
    assert_eq!(dump(&mut h, v), "{\"a\":n:3,\"b\":n:2}");
}

#[test]
fn protos_are_used() {
    let mut h = Heap::new().unwrap();
    let op = h.new_object(None).unwrap();
    let ap = h.new_object(None).unwrap();
    let opts = DecodeOptions {
        object_proto: Some(op),
        array_proto: Some(ap),
        ..DecodeOptions::default()
    };
    let b = encode(&T::Arr(vec![Some(T::Obj(vec![])), Some(T::Bytes(vec![1]))]));
    let v = decode(&mut h, &b, &opts).unwrap();
    let arr = v.as_object().unwrap();
    assert_eq!(h.get_prototype_of(arr), Some(ap));
    let el = |i| {
        h.get_own_property(arr, Atom::int(i).unwrap())
            .unwrap()
            .value()
            .unwrap()
    };
    assert_eq!(h.get_prototype_of(el(0).as_object().unwrap()), Some(op));
    assert_eq!(h.get_prototype_of(el(1).as_object().unwrap()), Some(ap));
}

#[test]
fn invalid_input() {
    let hdr = [b'D', b'J', b'W', 1];
    let with = |rest: &[u8]| {
        let mut b = hdr.to_vec();
        b.extend_from_slice(rest);
        b
    };
    assert_eq!(invalid(b""), (0, "unexpected end"));
    assert_eq!(invalid(b"XJW\x01\x01"), (0, "bad magic"));
    assert_eq!(invalid(b"DJW\x02\x01"), (3, "unsupported version"));
    assert_eq!(invalid(&hdr), (4, "unexpected end"));
    assert_eq!(invalid(&with(&[0x01, 0x01])), (5, "trailing bytes"));
    assert_eq!(invalid(&with(&[0x99])), (4, "unknown tag"));
    assert_eq!(invalid(&with(&[tag::HOLE])), (4, "hole outside an array"));
    assert_eq!(
        invalid(&with(&[tag::HOST_REF, 1])),
        (4, "host refs are not supported")
    );
    assert_eq!(invalid(&with(&[tag::INT32, 1, 2])).1, "unexpected end");
    assert_eq!(
        invalid(&with(&[tag::ARRAY, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F])).1,
        "varint overflow"
    );
    assert_eq!(
        invalid(&with(&[tag::ARRAY, 0x80, 0x80, 0x80, 0x80, 0x80, 0])).1,
        "varint too long"
    );
    assert_eq!(
        invalid(&with(&[tag::ARRAY, 100, 1])).1,
        "count exceeds the input"
    );
    assert_eq!(
        invalid(&with(&[tag::OBJECT, 2, 0, tag::STR_UTF8])).1,
        "count exceeds the input"
    );
    assert_eq!(
        invalid(&with(&[tag::STR_UTF8, 1, 2, 0xC3, 0x28])).1,
        "invalid UTF-8"
    );
    assert_eq!(
        invalid(&with(&[tag::STR_UTF8, 2, 3, 0xED, 0xA0, 0x80])).1,
        "invalid UTF-8"
    );
    assert_eq!(
        invalid(&with(&[tag::STR_UTF8, 1, 2, b'a', b'b'])).1,
        "string lengths do not match"
    );
    assert_eq!(
        invalid(&with(&[tag::STR_UTF8, 3, 2, b'a', b'b'])).1,
        "string lengths do not match"
    );
    assert_eq!(
        invalid(&with(&[tag::STR_UTF8, 3, 4, 0xC3, 0xA9, 0xC3, 0xA9])).1,
        "string lengths do not match"
    );
    assert_eq!(
        invalid(&with(&[tag::OBJECT, 1, 1, tag::NULL])).1,
        "key index out of range"
    );
    assert_eq!(
        invalid(&with(&[tag::OBJECT, 1, 0, tag::NULL, tag::NULL])).1,
        "expected a key string"
    );
    assert_eq!(
        invalid(&with(&[tag::OBJECT, 1, 0, tag::STR_UTF8, 2, 1, b'a', 0])).1,
        "string lengths do not match"
    );
}

/// A deterministic xorshift generator.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn random_value(r: &mut Rng, depth: u32) -> T {
    let words = ["", "a", "key", "é", "日本", "😀", "x_y", "0", "17"];
    match r.below(if depth > 3 { 6 } else { 9 }) {
        0 => [T::Undef, T::Null, T::Bool(true), T::Bool(false)][r.below(4) as usize].clone(),
        1 => T::Num([0.0, -0.0, 1.5, 300.0, -5.0, 1e300, f64::NAN][r.below(7) as usize]),
        2 => T::Num(r.below(100_000) as f64 - 50_000.0),
        3 | 4 => s(&format!(
            "{}{}",
            words[r.below(words.len() as u64) as usize],
            words[r.below(words.len() as u64) as usize]
        )),
        5 => T::Str16(vec![0x61, 0xD800 + r.below(0x800) as u16]),
        6 => T::Arr(
            (0..r.below(6))
                .map(|_| (r.below(8) != 0).then(|| random_value(r, depth + 1)))
                .collect(),
        ),
        7 => T::Obj(
            (0..r.below(6))
                .map(|_| {
                    (
                        format!(
                            "{}{}",
                            words[r.below(words.len() as u64) as usize],
                            r.below(4)
                        ),
                        random_value(r, depth + 1),
                    )
                })
                .collect(),
        ),
        _ => T::Bytes((0..r.below(5)).map(|_| r.next() as u8).collect()),
    }
}

#[test]
fn random_values_round_trip() {
    let mut r = Rng(0x1234_5678_9abc_def1);
    for _ in 0..3000 {
        let v = random_value(&mut r, 0);
        let mut v = v;
        // Repeated keys are legal input but make the expectation differ; dedupe them.
        dedupe(&mut v);
        check(v);
    }
}

fn dedupe(v: &mut T) {
    match v {
        T::Arr(items) => items.iter_mut().flatten().for_each(dedupe),
        T::Obj(props) => {
            let mut seen: Vec<String> = Vec::new();
            props.retain(|(k, _)| {
                let new = !seen.contains(k);
                seen.push(k.clone());
                new
            });
            props.iter_mut().for_each(|(_, v)| dedupe(v));
        }
        _ => {}
    }
}

/// Random and corrupted buffers never panic, and whatever decodes leaves a valid heap.
#[test]
fn fuzz_never_panics() {
    fuzz(0xdead_beef_cafe_f00d, 12_000);
}

/// A longer run: `cargo test -p djs-core --release fuzz_long -- --ignored`.
#[test]
#[ignore]
fn fuzz_long() {
    for seed in 1..=20 {
        fuzz(seed * 0x9e37_79b9_7f4a_7c15, 100_000);
    }
}

fn fuzz(seed: u64, iterations: usize) {
    let mut r = Rng(seed);
    let mut valid = Vec::new();
    for _ in 0..200 {
        let mut v = random_value(&mut r, 0);
        dedupe(&mut v);
        valid.push(encode(&v));
    }
    let mut ok = 0;
    for i in 0..iterations {
        let mut b = valid[i % valid.len()].clone();
        match r.below(6) {
            0 => {
                // Random bytes after a valid header.
                b.truncate(4);
                for _ in 0..r.below(40) {
                    b.push(r.next() as u8);
                }
            }
            1 => {
                let n = r.below(b.len() as u64 + 1) as usize;
                b.truncate(n);
            }
            2 => {
                for _ in 0..1 + r.below(3) {
                    let at = r.below(b.len() as u64) as usize;
                    b[at] ^= 1 << r.below(8);
                }
            }
            3 => {
                let at = r.below(b.len() as u64) as usize;
                b[at] = r.next() as u8;
            }
            4 => {
                let at = r.below(b.len() as u64 + 1) as usize;
                b.insert(
                    at,
                    [0x00, 0x7F, 0x80, 0xFF, 0x20, 0x21, 0x10][r.below(7) as usize],
                );
            }
            _ => {
                let at = r.below(b.len() as u64) as usize;
                b.remove(at);
            }
        }
        let mut h = Heap::with_limit(1 << 24).unwrap();
        if decode(&mut h, &b, &DecodeOptions::default()).is_ok() {
            ok += 1;
            h.verify().unwrap();
        }
    }
    // Some mutations still decode (e.g. a flipped number byte).
    assert!(ok > iterations / 60, "{ok}");
}

#[test]
fn heap_limits_are_errors() {
    let mut h = Heap::with_limit(1 << 16).unwrap();
    let b = encode(&s(&"x".repeat(200_000)));
    assert!(matches!(
        decode(&mut h, &b, &DecodeOptions::default()),
        Err(WireError::Heap(_))
    ));
}

/// Decodes what js/test/gen-cross.ts wrote (into `DJS_WIRE_CROSS`) and compares with the
/// node-side dump, and with `json::parse_utf8` of the same value where JSON can carry it.
/// The gate runs it (scripts/gate.sh).
#[test]
#[ignore]
fn wire_cross() {
    use crate::json::{ParseOptions, parse_utf8};
    let dir = std::env::var("DJS_WIRE_CROSS").expect("set DJS_WIRE_CROSS");
    let cases = std::fs::read_to_string(format!("{dir}/cases.tsv")).unwrap();
    let (mut n, mut with_json) = (0, 0);
    for line in cases.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        let wire = std::fs::read(format!("{dir}/{}.wire", f[0])).unwrap();
        let (mut h, v) = decode_ok(&wire);
        let got = dump(&mut h, v);
        assert_eq!(got, f[1], "case {}", f[0]);
        if f[2] == "1" {
            let text = std::fs::read(format!("{dir}/{}.json", f[0])).unwrap();
            let mut h2 = Heap::new().unwrap();
            let j = parse_utf8(&mut h2, &text, &ParseOptions::default()).unwrap();
            assert_eq!(dump(&mut h2, j), got, "case {} via JSON", f[0]);
            with_json += 1;
        }
        n += 1;
    }
    assert!(n > 1000 && with_json > 300, "{n} {with_json}");
    std::println!("{n} cases match ({with_json} also via JSON)");
}
