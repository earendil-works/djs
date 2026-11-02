//! Fake tools for pi sessions: a copy of snaplab's (src/main.rs), so results have the entropy
//! of real data (this repository's sources and docs).
#![allow(dead_code)]

use std::fmt::Write as _;
use std::path::Path;

pub const T0: f64 = 1_762_000_000_000.0;
pub const TOOLS: &[&str] = &[
    "search",
    "read",
    "ask",
    "shell",
    "logs",
    "lookup",
    "list_issues",
    "issue",
    "search_page",
    "status",
];

pub fn tools_json() -> String {
    let items: Vec<String> = TOOLS
        .iter()
        .map(|t| format!(r#"{{"name":"{t}","jsName":"{t}","description":"The {t} tool."}}"#))
        .collect();
    format!("[{}]", items.join(","))
}

fn js(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn arg_raw<'a>(args: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\":");
    let at = args.find(&pat)? + pat.len();
    Some(&args[at..])
}

fn arg_str(args: &str, key: &str) -> Option<String> {
    let r = arg_raw(args, key)?;
    let r = r.strip_prefix('"')?;
    Some(r[..r.find('"')?].to_string())
}

fn arg_num(args: &str, key: &str) -> Option<f64> {
    let r = arg_raw(args, key)?;
    let end = r.find([',', '}']).unwrap_or(r.len());
    r[..end].trim().parse().ok()
}

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

fn sentence(r: &mut Rng, n: usize) -> String {
    CORPUS.with(|c| {
        let w = &c.2;
        (0..n)
            .map(|_| w[r.below(w.len() as u64) as usize].as_str())
            .collect::<Vec<_>>()
            .join(" ")
    })
}

fn prose(r: &mut Rng, n: usize) -> String {
    CORPUS.with(|c| {
        let p = &c.1;
        let mut out = String::new();
        while out.len() < n {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(&p[r.below(p.len() as u64) as usize]);
        }
        out.truncate(
            out.char_indices()
                .map(|(i, _)| i)
                .take_while(|&i| i <= n)
                .last()
                .unwrap_or(0),
        );
        out
    })
}

thread_local! {
    static CORPUS: (Vec<(String, String)>, Vec<String>, Vec<String>) = load_corpus();
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 { a } else { gcd(b, a % b) }
}

pub struct Tools {
    pub failed_page2: bool,
}

// Tool results come from real source files and real prose (CORPUS), so they have the
// entropy of real data: synthetic repetitive text compresses unrealistically well.

fn load_corpus() -> (Vec<(String, String)>, Vec<String>, Vec<String>) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut files = vec![];
    let mut stack = vec![root.join("crates/djs-core/src")];
    while let Some(d) = stack.pop() {
        let mut es: Vec<_> = std::fs::read_dir(&d)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        es.sort();
        for e in es {
            if e.is_dir() {
                stack.push(e);
            } else if e.extension().is_some_and(|x| x == "rs") {
                let rel = e.strip_prefix(&root).unwrap().to_str().unwrap().to_string();
                files.push((rel, std::fs::read_to_string(&e).unwrap()));
            }
        }
    }
    files.sort();
    let mut prose = vec![];
    for f in ["docs/decisions.md", "ARCH.md", "TESTING.md", "docs/host.md"] {
        for l in std::fs::read_to_string(root.join(f)).unwrap().lines() {
            let l = l.trim();
            if l.len() > 30 && !l.starts_with('|') && !l.starts_with('#') {
                prose.push(l.to_string());
            }
        }
    }
    let mut words: Vec<String> = prose
        .iter()
        .flat_map(|l| l.split_whitespace())
        .filter(|w| w.chars().all(|c| c.is_ascii_alphabetic()) && w.len() > 2)
        .map(|w| w.to_lowercase())
        .collect();
    words.sort();
    words.dedup();
    (files, prose, words)
}

impl Tools {
    /// (ok, payload JSON) for a call.
    pub fn call(&mut self, name: &str, args: &str) -> (bool, String) {
        let mut r = Rng(0x9e37_79b9_7f4a_7c15 ^ (args.len() as u64 * 0x1000_0001));
        match name {
            "search" => {
                let limit = arg_num(args, "limit").unwrap_or(10.0) as usize;
                let items: Vec<String> = CORPUS.with(|c| {
                    (0..limit)
                        .map(|i| {
                            // A step coprime with the file count, so the hits are distinct files (0133).
                            let n = c.0.len();
                            let stride = (7..).find(|&s| gcd(s, n) == 1).unwrap();
                            let f = &c.0[(i * stride + 3) % n].0;
                            format!(
                                r#"{{"path":{},"score":{:.4}}}"#,
                                js(f),
                                1.0 / (i as f64 + 1.0)
                            )
                        })
                        .collect()
                });
                (true, format!("[{}]", items.join(",")))
            }
            "read" => {
                let path = arg_str(args, "path").unwrap_or_default();
                let text = CORPUS.with(|c| {
                    let f =
                        c.0.iter()
                            .find(|f| f.0 == path)
                            .map(|f| f.1.clone())
                            .unwrap_or_default();
                    f.lines().take(150).collect::<Vec<_>>().join("\n")
                });
                (
                    true,
                    format!(r#"{{"path":{},"text":{}}}"#, js(&path), js(&text)),
                )
            }
            "ask" => {
                let opt = arg_raw(args, "options")
                    .and_then(|r| r.strip_prefix("[\""))
                    .map(|r| r[..r.find('"').unwrap()].to_string())
                    .unwrap_or("yes".into());
                (true, format!(r#"{{"choice":{}}}"#, js(&opt)))
            }
            "shell" => {
                let out: Vec<String> = CORPUS.with(|c| {
                    (0..40)
                        .map(|i| {
                            format!(
                                "[{i:02}/40] Compiling {} ({})",
                                c.0[r.below(c.0.len() as u64) as usize].0,
                                sentence(&mut r, 3)
                            )
                        })
                        .collect()
                });
                (
                    true,
                    format!(r#"{{"code":0,"stdout":{}}}"#, js(&out.join("\n"))),
                )
            }
            "logs" => {
                let n = arg_num(args, "lines").unwrap_or(1000.0) as usize;
                let mut s = String::new();
                let routes = ["users", "orders", "items", "carts", "sessions"];
                for i in 0..n {
                    let err = r.below(20) == 0;
                    let _ = write!(
                        s,
                        "2026-11-03T10:{:02}:{:02}.{:03}Z {} req={:08x} route=/v1/{}/{} ms={}",
                        (i / 3600) % 60,
                        (i / 60) % 60,
                        i % 1000,
                        if err { "ERROR" } else { "INFO" },
                        r.next() as u32,
                        routes[r.below(5) as usize],
                        r.below(100000),
                        r.below(900)
                    );
                    if err {
                        let _ = write!(
                            s,
                            " code=E{} user={} msg=\"{}\"",
                            100 + r.below(17),
                            r.below(100000),
                            sentence(&mut r, 6)
                        );
                    }
                    s.push('\n');
                }
                (true, js(&s))
            }
            "lookup" => (
                true,
                format!(
                    r#"{{"owner":"team-{}","runbook":"https://wiki.example.com/runbooks/{}"}}"#,
                    r.below(9),
                    r.next() % 1000
                ),
            ),
            "list_issues" => {
                let n = arg_num(args, "limit").unwrap_or(100.0) as usize;
                let labels = [
                    "bug", "feature", "p1", "p2", "p3", "api", "ui", "infra", "docs", "flaky",
                    "perf", "security",
                ];
                let items: Vec<String> = (0..n)
                    .map(|i| {
                        let k = 1 + r.below(3) as usize;
                        let ls: Vec<String> = (0..k).map(|_| js(labels[r.below(12) as usize])).collect();
                        format!(
                            r#"{{"number":{},"title":{},"state":"open","labels":[{}],"user":{{"login":"user{}","id":{}}},"points":{},"created_at":"2026-{:02}-{:02}T12:00:00Z","body":{}}}"#,
                            1000 + i,
                            js(&sentence(&mut r, 7)),
                            ls.join(","),
                            r.below(80),
                            r.below(1_000_000),
                            1 + r.below(8),
                            1 + r.below(12),
                            1 + r.below(28),
                            { let n = 160 + r.below(300) as usize; js(&prose(&mut r, n)) }
                        )
                    })
                    .collect();
                (true, format!("[{}]", items.join(",")))
            }
            "issue" => {
                let num = arg_num(args, "number").unwrap_or(0.0) as u64;
                let k = 2 + r.below(6);
                let cs: Vec<String> = (0..k)
                    .map(|_| {
                        format!(r#"{{"author":"user{}","body":{}}}"#, r.below(80), {
                            let n = 60 + r.below(400) as usize;
                            js(&prose(&mut r, n))
                        })
                    })
                    .collect();
                (
                    true,
                    format!(r#"{{"number":{num},"comments":[{}]}}"#, cs.join(",")),
                )
            }
            "search_page" => {
                let page = arg_num(args, "cursor").unwrap_or(0.0) as u64;
                if page == 2 && !self.failed_page2 {
                    self.failed_page2 = true;
                    return (false, "rate limited".into());
                }
                let owners = ["alice", "bob", "carol", "dave", "erin", "frank", "grace"];
                let items: Vec<String> = (0..20)
                    .map(|i| {
                        let id = (page * 20 + i) % 90;
                        format!(
                            r#"{{"id":{id},"owner":"{}","title":{}}}"#,
                            owners[(id % 7) as usize],
                            js(&sentence(&mut r, 6))
                        )
                    })
                    .collect();
                let next = if page < 5 {
                    format!("{}", page + 1)
                } else {
                    "null".into()
                };
                (
                    true,
                    format!(r#"{{"items":[{}],"next":{next}}}"#, items.join(",")),
                )
            }
            "status" => (true, r#"{"state":"open"}"#.into()),
            _ => (false, format!("unknown tool {name}")),
        }
    }
}

