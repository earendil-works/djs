//! The boundary datasets of 0018/0021 (`js/bench/wire.bench.ts`), generated natively: a
//! small value tree that writes both JSON and the wire format (docs/wire.md).

use std::fmt::Write;

#[derive(Clone, Debug)]
pub enum Data {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Data>),
    Obj(Vec<(&'static str, Data)>),
}

impl Data {
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        self.json_into(&mut out);
        out
    }

    fn json_into(&self, out: &mut String) {
        match self {
            Data::Null => out.push_str("null"),
            Data::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Data::Num(n) => write!(out, "{n}").unwrap(),
            Data::Str(s) => json_string(s, out),
            Data::Arr(items) => {
                out.push('[');
                for (i, it) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    it.json_into(out);
                }
                out.push(']');
            }
            Data::Obj(props) => {
                out.push('{');
                for (i, (k, v)) in props.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    json_string(k, out);
                    out.push(':');
                    v.json_into(out);
                }
                out.push('}');
            }
        }
    }

    /// The wire encoding, like `js/src/wire.ts` writes it (keys through the key table).
    pub fn to_wire(&self) -> Vec<u8> {
        let mut w = Wire {
            out: b"DJW\x01".to_vec(),
            keys: Vec::new(),
        };
        w.value(self);
        w.out
    }
}

fn json_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => write!(out, "\\u{:04x}", c as u32).unwrap(),
            c => out.push(c),
        }
    }
    out.push('"');
}

struct Wire {
    out: Vec<u8>,
    keys: Vec<&'static str>,
}

impl Wire {
    fn varint(&mut self, mut v: usize) {
        while v >= 0x80 {
            self.out.push((v as u8 & 0x7F) | 0x80);
            v >>= 7;
        }
        self.out.push(v as u8);
    }

    fn string(&mut self, s: &str) {
        self.out.push(0x10);
        self.varint(s.encode_utf16().count());
        self.varint(s.len());
        self.out.extend_from_slice(s.as_bytes());
    }

    fn value(&mut self, v: &Data) {
        match v {
            Data::Null => self.out.push(0x01),
            Data::Bool(b) => self.out.push(if *b { 0x03 } else { 0x02 }),
            Data::Num(n) => {
                let i = *n as i32;
                if i as f64 == *n && !(i == 0 && n.is_sign_negative()) {
                    if (-128..=127).contains(&i) {
                        self.out.push(0x07);
                        self.out.push(i as i8 as u8);
                    } else {
                        self.out.push(0x04);
                        self.out.extend_from_slice(&i.to_le_bytes());
                    }
                } else {
                    self.out.push(0x05);
                    self.out.extend_from_slice(&n.to_le_bytes());
                }
            }
            Data::Str(s) => self.string(s),
            Data::Arr(items) => {
                self.out.push(0x20);
                self.varint(items.len());
                for it in items {
                    self.value(it);
                }
            }
            Data::Obj(props) => {
                self.out.push(0x21);
                self.varint(props.len());
                for (k, v) in props {
                    match self.keys.iter().position(|x| x == k) {
                        Some(i) => self.varint(i + 1),
                        None => {
                            self.keys.push(k);
                            self.out.push(0);
                            self.string(k);
                        }
                    }
                    self.value(v);
                }
            }
        }
    }
}

/// wire.bench.ts's LCG, so the shapes match (not the exact bytes).
struct Rng(u32);

impl Rng {
    fn next(&mut self, n: u32) -> u32 {
        self.0 = self.0.wrapping_mul(1103515245).wrapping_add(12345);
        self.0 % n
    }

    fn pick<'a>(&mut self, a: &[&'a str]) -> &'a str {
        a[self.next(a.len() as u32) as usize]
    }
}

const WORDS: &[&str] = &[
    "alpha", "beta", "gamma", "delta", "config", "server", "request", "handler", "error", "value",
    "größe", "naïve", "日本", "user", "data", "index", "parse", "result",
];

fn sentence(r: &mut Rng, n: usize) -> String {
    (0..n).map(|_| r.pick(WORDS)).collect::<Vec<_>>().join(" ")
}

fn similar_objects(r: &mut Rng, n: usize) -> Data {
    Data::Arr(
        (0..n)
            .map(|i| {
                let parent = if r.next(4) == 0 {
                    Data::Null
                } else {
                    Data::Num(r.next(n as u32) as f64)
                };
                Data::Obj(vec![
                    ("id", Data::Num(i as f64)),
                    (
                        "uuid",
                        Data::Str(format!(
                            "{:x}-{:x}",
                            (i as u32).wrapping_mul(2654435761),
                            r.next(65536)
                        )),
                    ),
                    (
                        "name",
                        Data::Str(format!("{} {}", r.pick(WORDS), r.pick(WORDS))),
                    ),
                    ("email", Data::Str(format!("user{i}@example.com"))),
                    ("active", Data::Bool(r.next(2) == 0)),
                    ("score", Data::Num(r.next(100000) as f64 / 100.0)),
                    (
                        "tags",
                        Data::Arr(vec![
                            Data::Str(r.pick(WORDS).into()),
                            Data::Str(r.pick(WORDS).into()),
                        ]),
                    ),
                    (
                        "created",
                        Data::Str(format!(
                            "2024-{:02}-{:02}T10:00:00Z",
                            1 + r.next(12),
                            1 + r.next(28)
                        )),
                    ),
                    ("parent", parent),
                ])
            })
            .collect(),
    )
}

fn source_text(r: &mut Rng, bytes: usize) -> String {
    let mut out = String::new();
    while out.len() < bytes {
        let indent = " ".repeat(r.next(4) as usize * 4);
        let (a, b, n) = (r.pick(WORDS), r.pick(WORDS), r.next(100));
        let comment = sentence(r, 4);
        writeln!(out, "{indent}let {a} = {b}({n}); // {comment}").unwrap();
    }
    out
}

fn tool_results(r: &mut Rng, hits: usize, file_bytes: usize) -> Data {
    let results = (0..hits)
        .map(|i| {
            let path = format!("src/{}/{}_{i}.ts", r.pick(WORDS), r.pick(WORDS));
            let line = r.next(5000);
            let words = 8 + r.next(12) as usize;
            let text = sentence(r, words);
            let score = r.next(1000) as f64 / 1000.0;
            let matches = (0..1 + r.next(3))
                .map(|_| {
                    Data::Obj(vec![
                        ("start", Data::Num(r.next(80) as f64)),
                        ("end", Data::Num((80 + r.next(40)) as f64)),
                    ])
                })
                .collect();
            Data::Obj(vec![
                ("path", Data::Str(path)),
                ("line", Data::Num(line as f64)),
                ("text", Data::Str(text)),
                ("score", Data::Num(score)),
                ("matches", Data::Arr(matches)),
            ])
        })
        .collect();
    Data::Obj(vec![
        ("query", Data::Str("handler error".into())),
        ("results", Data::Arr(results)),
        (
            "stats",
            Data::Obj(vec![
                ("files", Data::Num(12873.0)),
                ("matched", Data::Num(hits as f64)),
                ("elapsedMs", Data::Num(412.5)),
                ("truncated", Data::Bool(true)),
            ]),
        ),
        (
            "file",
            Data::Obj(vec![
                ("path", Data::Str("src/server/handler.ts".into())),
                ("content", Data::Str(source_text(r, file_bytes))),
            ]),
        ),
    ])
}

fn numbers(r: &mut Rng, n: usize) -> Data {
    Data::Arr(
        (0..n)
            .map(|i| {
                if i % 2 == 1 {
                    Data::Num(r.next(1_000_000) as f64)
                } else {
                    Data::Num(r.next(1_000_000_000) as f64 / 1000.0)
                }
            })
            .collect(),
    )
}

/// The five datasets, at full size (`small` for quick mode).
pub fn datasets(small: bool) -> Vec<(&'static str, Data)> {
    let k = if small { 100 } else { 1 };
    let mut r = Rng(7);
    vec![
        ("10k-objects", similar_objects(&mut r, 10_000 / k)),
        ("1mib-string", Data::Str("x".repeat((1 << 20) / k))),
        (
            "1mib-text-unicode",
            Data::Str(source_text(&mut r, (1 << 20) / k)),
        ),
        ("tool-results", tool_results(&mut r, 2000 / k, 200_000 / k)),
        ("500k-numbers", numbers(&mut r, 500_000 / k)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_and_wire_agree() {
        use djs_core::heap::Heap;
        use djs_core::json::{ParseOptions, parse_utf8, stringify_fast};
        use djs_core::wire::{DecodeOptions, decode};
        for (name, d) in datasets(true) {
            let mut h = Heap::new().unwrap();
            let a = decode(&mut h, &d.to_wire(), &DecodeOptions::default()).unwrap();
            let b = parse_utf8(&mut h, d.to_json().as_bytes(), &ParseOptions::default()).unwrap();
            let sa = stringify_fast(&mut h, a, &[]).unwrap().unwrap();
            let sb = stringify_fast(&mut h, b, &[]).unwrap().unwrap();
            assert_eq!(h.str_to_string(sa), h.str_to_string(sb), "{name}");
        }
    }
}
