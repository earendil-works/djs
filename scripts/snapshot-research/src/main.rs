//! snaplab (issue 0088): runs pi codemode scripts on djs with pi's prelude and fake tools,
//! takes VM images at every idle point (waiting on tools), and measures what alternative
//! snapshot designs would cost on them. See README.md; run.sh drives it.
//!
//!   snaplab <out-dir> <pi-prelude.js> [case.js ...]

mod analyze;
mod logical;
mod pi;

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use djs_core::vm::Vm;

const T0: f64 = 1_762_000_000_000.0;
const TOOLS: &[&str] = &[
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

fn tools_json() -> String {
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

struct Tools {
    failed_page2: bool,
}

// Tool results come from real source files and real prose (CORPUS), so they have the
// entropy of real data: synthetic repetitive text compresses unrealistically well.

fn load_corpus() -> (Vec<(String, String)>, Vec<String>, Vec<String>) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
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
    fn call(&mut self, name: &str, args: &str) -> (bool, String) {
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

fn zstd(data: &[u8], args: &[&str]) -> usize {
    let f = std::env::temp_dir().join(format!("snaplab-{}.bin", std::process::id()));
    std::fs::write(&f, data).unwrap();
    let out = Command::new("zstd")
        .args(args)
        .args(["-q", "-c", "--long=27"])
        .arg(&f)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout.len()
}

fn tool(cmd: &str, args: &[&str], data: &[u8]) -> usize {
    let mut c = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut i = c.stdin.take().unwrap();
    let d = data.to_vec();
    let t = std::thread::spawn(move || i.write_all(&d).unwrap());
    let out = c.wait_with_output().unwrap().stdout.len();
    t.join().unwrap();
    out
}

fn collected(raw: &[u8]) -> Vec<u8> {
    let mut vm = Vm::from_bytes(raw.to_vec(), pi::opts()).unwrap();
    vm.collect().unwrap();
    vm.bytes().to_vec()
}

struct Point {
    delivered: usize,
    pending: usize,
    raw: Vec<u8>,
    img: Vec<u8>,
    journal: usize,
}

struct Run {
    code: String,
    points: Vec<Point>,
    journal: Vec<(u32, bool, String)>,
    output: usize,
    done: Option<(bool, String)>,
}

fn run_script(code: &str, prelude: &str) -> Run {
    let mut s = pi::Session::prepare(prelude, &tools_json(), T0);
    s.start(code, 42);
    let mut tools = Tools {
        failed_page2: false,
    };
    let mut points = vec![];
    let mut journal = vec![];
    let mut journal_bytes = code.len() + 64;
    let mut now = T0;
    loop {
        if s.ev.done.is_some() || s.ev.calls.is_empty() {
            break;
        }
        let raw = s.vm.bytes().to_vec();
        let img = collected(&raw);
        points.push(Point {
            delivered: journal.len(),
            pending: s.ev.calls.len(),
            raw,
            img,
            journal: journal_bytes,
        });
        let call = s.ev.calls.remove(0);
        let (ok, payload) = tools.call(&call.name, call.args.as_deref().unwrap_or(""));
        journal_bytes += payload.len() + 24;
        now += 1500.0;
        s.deliver(call.id, ok, &payload, now);
        journal.push((call.id, ok, payload));
    }
    Run {
        code: code.into(),
        points,
        journal,
        output: s.ev.output.iter().map(|o| o.len()).sum(),
        done: s.ev.done.take(),
    }
}

/// Replays `run`'s journal up to `k` results from scratch (or from the prelude image).
fn replay(run: &Run, k: usize, prelude: &str, image: Option<(&[u8], &pi::Session)>) -> f64 {
    let t = Instant::now();
    let mut s = match image {
        None => pi::Session::prepare(prelude, &tools_json(), T0),
        // handles are part of the image: the session's handle numbers carry over
        Some((img, like)) => pi::Session::from_image(img, like),
    };
    s.start(&run.code, 42);
    let mut now = T0;
    for (id, ok, payload) in run.journal.iter().take(k) {
        now += 1500.0;
        s.deliver(*id, *ok, payload, now);
    }
    t.elapsed().as_secs_f64() * 1000.0
}

fn kb(n: usize) -> String {
    format!("{:.1}", n as f64 / 1024.0)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out_dir = PathBuf::from(args.next().expect("out dir"));
    std::fs::create_dir_all(&out_dir).unwrap();
    let prelude = std::fs::read_to_string(args.next().expect("pi prelude (.js)")).unwrap();
    let scripts: Vec<String> = args.collect();

    // References: the engine's base image, and the image right after pi's prelude (what
    // the adapter caches, and what a restore can recompute from the prelude's inputs).
    let base = djs_prelude::BASE.to_vec();
    let pp = pi::Session::prepare(&prelude, &tools_json(), T0);
    let pp_raw = pp.vm.bytes().to_vec();
    let pp_img = collected(&pp_raw);
    std::fs::write(out_dir.join("base.img"), &base).unwrap();
    std::fs::write(out_dir.join("post-prelude.img"), &pp_img).unwrap();
    let base_h = analyze::load(&base);
    let pp_h = analyze::load(&pp_img);
    let base_paths = analyze::paths(&base_h);
    let pp_paths = analyze::paths(&pp_h);

    let mut report = String::new();
    let _ = writeln!(
        report,
        "base image {} KB (zstd -19 {} KB); post-prelude image raw {} KB, collected {} KB (zstd -19 {} KB, vs base patch-from {} KB)",
        kb(base.len()),
        kb(zstd(&base, &["-19"])),
        kb(pp_raw.len()),
        kb(pp_img.len()),
        kb(zstd(&pp_img, &["-19"])),
        kb(zstd(
            &pp_img,
            &[
                "-19",
                "--patch-from",
                out_dir.join("base.img").to_str().unwrap()
            ]
        ))
    );
    {
        let m = analyze::match_graphs(&base_h, &pp_h);
        let (d, mutated) = analyze::delta_estimate(&base_h, &pp_h, &m);
        let _ = writeln!(report, "post-prelude vs base: {d:?}");
        let st = logical::encode(
            &pp_h,
            Some(logical::Known {
                m: &m,
                mutated: &mutated,
                paths: &base_paths,
            }),
        );
        let _ = writeln!(
            report,
            "post-prelude logical vs base: total {} KB (strings {} KB in {}, paths {} KB in {}, shapes {} KB), new units {}, patches {}, zstd {} KB",
            kb(st.total),
            kb(st.strings),
            st.string_count,
            kb(st.paths),
            st.path_count,
            kb(st.shapes),
            st.new_units,
            st.patch_units,
            kb(zstd(&st.bytes, &["-19"]))
        );
        let _ = writeln!(
            report,
            "  records: {:?}\n  patches: {:?}",
            st.records, st.patches
        );
        let (k, c) = analyze::histogram(&pp_h);
        let _ = writeln!(report, "post-prelude kinds: {k:?}\n  classes: {c:?}");
        let _ = writeln!(
            report,
            "post-prelude code: {:?}",
            analyze::code_partition(&pp_h)
        );
        let (k, c) = analyze::histogram(&base_h);
        let _ = writeln!(report, "base kinds: {k:?}\n  classes: {c:?}");
        let _ = writeln!(report, "base code: {:?}", analyze::code_partition(&base_h));
    }
    // restore costs of a raw image
    {
        let t = Instant::now();
        for _ in 0..20 {
            let _ = Vm::from_bytes(pp_img.clone(), pi::opts()).unwrap();
        }
        let verified = t.elapsed().as_secs_f64() * 1000.0 / 20.0;
        let t = Instant::now();
        for _ in 0..20 {
            let _ = djs_core::heap::Heap::from_bytes_trusted(pp_img.clone(), 1 << 30).unwrap();
        }
        let trusted = t.elapsed().as_secs_f64() * 1000.0 / 20.0;
        let t = Instant::now();
        for _ in 0..5 {
            let _ = pi::Session::prepare(&prelude, &tools_json(), T0);
        }
        let prep = t.elapsed().as_secs_f64() * 1000.0 / 5.0;
        let _ = writeln!(
            report,
            "restore post-prelude image: verified {verified:.2} ms, trusted copy {trusted:.3} ms; recomputing it (base + prelude) {prep:.2} ms"
        );
    }

    let mut csv = String::from(
        "script,idle,delivered,pending,raw,img,journal,new_vs_pp,mut_vs_pp,pages_vs_pp,changed_words_pp,new_vs_base,mut_vs_base,pages_vs_base,logical_pp,logical_pp_strings,logical_base,activations,max_operands\n",
    );
    for path in &scripts {
        let code = std::fs::read_to_string(path).unwrap();
        let name = Path::new(path)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let t = Instant::now();
        let run = run_script(&code, &prelude);
        let run_ms = t.elapsed().as_secs_f64() * 1000.0;
        let _ = writeln!(
            report,
            "\n## {name}: {} idle points, {} results ({} KB of payloads), output {} B, done {:?}, run incl. snapshots {run_ms:.0} ms",
            run.points.len(),
            run.journal.len(),
            kb(run.journal.iter().map(|j| j.2.len()).sum()),
            run.output,
            run.done
                .as_ref()
                .map(|d| (d.0, d.1.chars().take(100).collect::<String>()))
        );
        let n = run.points.len();
        let max_i = (0..n).max_by_key(|&i| run.points[i].img.len()).unwrap_or(0);
        let mut selected: Vec<usize> = vec![0, max_i, n.saturating_sub(1)];
        selected.sort();
        selected.dedup();
        for (i, p) in run.points.iter().enumerate() {
            let h = analyze::load(&p.img);
            let m_pp = analyze::match_graphs(&pp_h, &h);
            let (d_pp, mut_pp) = analyze::delta_estimate(&pp_h, &h, &m_pp);
            let m_b = analyze::match_graphs(&base_h, &h);
            let (d_b, mut_b) = analyze::delta_estimate(&base_h, &h, &m_b);
            let l_pp = logical::encode(
                &h,
                Some(logical::Known {
                    m: &m_pp,
                    mutated: &mut_pp,
                    paths: &pp_paths,
                }),
            );
            let l_b = logical::encode(
                &h,
                Some(logical::Known {
                    m: &m_b,
                    mutated: &mut_b,
                    paths: &base_paths,
                }),
            );
            let acts = analyze::activations(&h);
            let max_ops = acts.iter().map(|a| a.operands).max().unwrap_or(0);
            let _ = writeln!(
                csv,
                "{name},{i},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                p.delivered,
                p.pending,
                p.raw.len(),
                p.img.len(),
                p.journal,
                d_pp.new_bytes,
                d_pp.mutated_bytes,
                d_pp.dirty_pages,
                d_pp.changed_words,
                d_b.new_bytes,
                d_b.mutated_bytes,
                d_b.dirty_pages,
                l_pp.total,
                l_pp.strings,
                l_b.total,
                acts.len(),
                max_ops
            );
            if !selected.contains(&i) {
                continue;
            }
            let f = out_dir.join(format!("{name}-{i}.img"));
            std::fs::write(&f, &p.img).unwrap();
            let _ = writeln!(
                report,
                "\n### {name} idle {i} (after {} results, {} calls pending)",
                p.delivered, p.pending
            );
            let _ = writeln!(
                report,
                "image: raw (pi's snapshot, uncollected) {} KB, collected {} KB; zstd -3 {} KB, zstd -19 {} KB, gzip -9 {} KB, brotli -q 11 {} KB, xz -9 {} KB",
                kb(p.raw.len()),
                kb(p.img.len()),
                kb(zstd(&p.img, &["-3"])),
                kb(zstd(&p.img, &["-19"])),
                kb(tool("gzip", &["-9", "-c"], &p.img)),
                kb(tool("brotli", &["-q", "11", "-c"], &p.img)),
                kb(tool("xz", &["-9", "-c"], &p.img))
            );
            let _ = writeln!(
                report,
                "byte delta (zstd -19 --patch-from): vs post-prelude {} KB, vs base {} KB; raw (uncollected) vs post-prelude {} KB",
                kb(zstd(
                    &p.img,
                    &[
                        "-19",
                        "--patch-from",
                        out_dir.join("post-prelude.img").to_str().unwrap()
                    ]
                )),
                kb(zstd(
                    &p.img,
                    &[
                        "-19",
                        "--patch-from",
                        out_dir.join("base.img").to_str().unwrap()
                    ]
                )),
                kb(zstd(
                    &p.raw,
                    &[
                        "-19",
                        "--patch-from",
                        out_dir.join("post-prelude.img").to_str().unwrap()
                    ]
                ))
            );
            let _ = writeln!(
                report,
                "old-gen delta vs post-prelude: new {} KB in {} cells, mutated {} cells ({} KB, {} changed words), dirty pages {}/{} -> page delta {} KB, cell delta {} KB",
                kb(d_pp.new_bytes as usize),
                d_pp.new_cells,
                d_pp.mutated_cells,
                kb(d_pp.mutated_bytes as usize),
                d_pp.changed_words,
                d_pp.dirty_pages,
                d_pp.ref_pages,
                kb((d_pp.new_bytes + d_pp.dirty_pages * 4096) as usize),
                kb((d_pp.new_bytes + d_pp.mutated_bytes) as usize)
            );
            let _ = writeln!(
                report,
                "  new by kind: {:?}\n  mutated by kind: {:?}",
                d_pp.new_by_kind, d_pp.mutated_by_kind
            );
            let _ = writeln!(
                report,
                "old-gen delta vs base: new {} KB in {} cells, mutated {} cells ({} KB), dirty pages {}/{} -> page delta {} KB",
                kb(d_b.new_bytes as usize),
                d_b.new_cells,
                d_b.mutated_cells,
                kb(d_b.mutated_bytes as usize),
                d_b.dirty_pages,
                d_b.ref_pages,
                kb((d_b.new_bytes + d_b.dirty_pages * 4096) as usize)
            );
            for (label, st) in [("post-prelude", &l_pp), ("base", &l_b)] {
                let _ = writeln!(
                    report,
                    "logical vs {label}: total {} KB (strings {} KB in {}, paths {} KB in {}, shapes {} KB), new units {}, patches {}, zstd -19 {} KB",
                    kb(st.total),
                    kb(st.strings),
                    st.string_count,
                    kb(st.paths),
                    st.path_count,
                    kb(st.shapes),
                    st.new_units,
                    st.patch_units,
                    kb(zstd(&st.bytes, &["-19"]))
                );
                let _ = writeln!(
                    report,
                    "  records: {:?}\n  patches: {:?}",
                    st.records, st.patches
                );
            }
            let jbytes: Vec<u8> = {
                let mut b = run.code.clone().into_bytes();
                for (id, ok, pl) in run.journal.iter().take(p.delivered) {
                    b.extend_from_slice(
                        format!("{{\"seq\":{id},\"ok\":{ok},\"now\":0}}").as_bytes(),
                    );
                    b.extend_from_slice(pl.as_bytes());
                }
                b
            };
            let _ = writeln!(
                report,
                "journal (code + {} results): {} KB, zstd -19 {} KB",
                p.delivered,
                kb(jbytes.len()),
                kb(zstd(&jbytes, &["-19"]))
            );
            if i > 0 {
                let prev = out_dir.join("prev.img");
                std::fs::write(&prev, &run.points[i - 1].img).unwrap();
                let _ = writeln!(
                    report,
                    "incremental: zstd -19 --patch-from the previous idle image: {} KB",
                    kb(zstd(
                        &p.img,
                        &["-19", "--patch-from", prev.to_str().unwrap()]
                    ))
                );
            }
            let (k, c) = analyze::histogram(&h);
            let _ = writeln!(report, "kinds: {k:?}\n  classes: {c:?}");
            let _ = writeln!(report, "code: {:?}", analyze::code_partition(&h));
            let _ = writeln!(report, "internals: {:?}", analyze::internals(&h));
            for a in &acts {
                let _ = writeln!(report, "  activation {a:?}");
            }
            let t_scratch = replay(&run, p.delivered, &prelude, None);
            let t_image = replay(&run, p.delivered, &prelude, Some((&pp_raw, &pp)));
            let t = Instant::now();
            let _ = Vm::from_bytes(p.img.clone(), pi::opts()).unwrap();
            let t_restore = t.elapsed().as_secs_f64() * 1000.0;
            let _ = writeln!(
                report,
                "restore: raw image (verified) {t_restore:.2} ms; replay of {} results from scratch {t_scratch:.1} ms, from the prelude image {t_image:.1} ms",
                p.delivered
            );
        }
        let awaits = analyze::await_depths(
            &analyze::load(&run.points.last().map(|p| p.img.clone()).unwrap_or_default()),
            "codemode.js",
        );
        for b in analyze::binding_names(
            &analyze::load(&run.points.last().map(|p| p.img.clone()).unwrap_or_default()),
            "codemode.js",
        ) {
            let _ = writeln!(report, "  template {b}");
        }
        let nontrivial: Vec<_> = awaits.iter().filter(|(_, d)| *d > 0).collect();
        let _ = writeln!(
            report,
            "static suspension points in codemode.js: {} ({} with operands below the awaited value: {:?})",
            awaits.len(),
            nontrivial.len(),
            nontrivial
        );
    }
    std::fs::write(out_dir.join("points.csv"), &csv).unwrap();
    std::fs::write(out_dir.join("report.txt"), &report).unwrap();
    print!("{report}");
}
