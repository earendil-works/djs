//! Benchmarks of djs-core's components. The scenarios and inputs are the ones the
//! issues that built them measured (0003 lexer, 0004 num, 0009 gc, 0011 regexp,
//! 0012 json), so the numbers stay comparable with their logs.

use std::hint::black_box;
use std::path::PathBuf;

use djs_core::heap::{Atom, Heap, atoms};
use djs_core::json::{ParseOptions, parse, parse_utf8, stringify_fast};
use djs_core::lexer::{Kw, LexError, Lexer, Token, TokenKind};
use djs_core::num::{string_to_number, to_fixed, to_precision, to_string, to_string_radix};
use djs_core::regexp::{ExecLimits, ExecResult, Flags, Input, compile, exec};
use djs_core::value::Value;

use crate::runner::{Runner, Unit};

/// The pinned typescript.js used for lexer throughput (fetched by scripts/bench.sh).
pub const TYPESCRIPT_JS: &str = "typescript-5.9.3.js";

/// `vendor/bench` of the primary checkout (shared by worktrees), or `$DJS_BENCH_VENDOR`.
pub fn vendor_dir() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("DJS_BENCH_VENDOR") {
        return Some(PathBuf::from(p));
    }
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()
        .ok()?;
    let common = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    Some(common.parent()?.join("vendor/bench"))
}

// ---- lexer ----------------------------------------------------------------------------

pub fn lexer(r: &mut Runner) {
    let synthetic = synthetic_js(r.scale(2_000_000, 20_000));
    r.bytes("lex/synthetic", synthetic.len(), || {
        black_box(lex_all(&synthetic));
    });
    let name = "lex/typescript.js";
    if !r.wants(name) {
        return;
    }
    let path = vendor_dir().map(|d| d.join(TYPESCRIPT_JS));
    match path.as_ref().and_then(|p| std::fs::read(p).ok()) {
        Some(src) if !r.config.quick => r.bytes(name, src.len(), || {
            black_box(lex_all(&src));
        }),
        Some(_) => r.skip(name, "not in quick mode"),
        None => r.skip(
            name,
            format!("{TYPESCRIPT_JS} not in vendor/bench (run scripts/bench.sh)"),
        ),
    }
}

// ---- compiler ---------------------------------------------------------------------------

/// Compile throughput (0092): the synthetic lexer input as a Script, and typescript.js
/// (9 MB, one huge closure), also with lazy functions (D2990: a function that does not run
/// at once is compiled at its first call, so the load compiles only the top level). Each
/// compile gets a fresh heap, created untimed.
pub fn compiler(r: &mut Runner) {
    use djs_core::compiler::{CompileOptions, compile, compile_lazy_unit};
    use std::cell::RefCell;
    let run = |r: &mut Runner, name: &str, src: &[u8], lazy: bool| {
        let heap = RefCell::new(Heap::new().unwrap());
        let opts = CompileOptions::script("bench.js");
        r.bytes_with_setup(
            name,
            src.len(),
            || *heap.borrow_mut() = Heap::new().unwrap(),
            || {
                let heap = &mut heap.borrow_mut();
                black_box(if lazy {
                    compile_lazy_unit(heap, src, &opts).unwrap()
                } else {
                    compile(heap, src, &opts).unwrap()
                });
            },
        );
    };
    // (Without `export`: a Script, not a module.)
    let synthetic = String::from_utf8(synthetic_js(r.scale(1_000_000, 20_000)))
        .unwrap()
        .replace("export ", "");
    run(r, "compile/synthetic", synthetic.as_bytes(), false);
    for (name, lazy) in [
        ("compile/typescript.js", false),
        ("compile/typescript.js lazy", true),
    ] {
        if !r.wants(name) {
            continue;
        }
        match vendor_dir().and_then(|d| std::fs::read(d.join(TYPESCRIPT_JS)).ok()) {
            Some(src) if !r.config.quick => run(r, name, &src, lazy),
            Some(_) => r.skip(name, "not in quick mode"),
            None => r.skip(
                name,
                format!("{TYPESCRIPT_JS} not in vendor/bench (run scripts/bench.sh)"),
            ),
        }
    }
}

/// Generated JS with a realistic token mix (identifiers, keywords, numbers, strings,
/// templates, regexes, comments), about `size` bytes.
pub fn synthetic_js(size: usize) -> Vec<u8> {
    const CHUNK: &str = r#"// Compute a summary of the items.
export async function summarize(items, { limit = 10, verbose } = {}) {
  const out = [];
  let total = 0, count = 0x1f;
  for (const item of items) {
    if (item?.size > 1024 * 1.5e3 && /^[a-z_]\w*\.(ts|js)$/i.test(item.name)) {
      total += item.size; count++;
      out.push(`${item.name}: ${(item.size / 1024).toFixed(2)} KiB`);
    } else if (verbose) {
      console.log("skipping \"%s\"", item.name, 'n/a');
    }
  }
  /* Sort by size, largest first. */
  return { total, count, lines: out.sort((a, b) => b.length - a.length).slice(0, limit) };
}
"#;
    let mut out = Vec::with_capacity(size + CHUNK.len());
    let mut i = 0;
    while out.len() < size {
        out.extend_from_slice(
            CHUNK
                .replace("summarize", &format!("summarize{i}"))
                .as_bytes(),
        );
        i += 1;
    }
    out
}

/// Lexes a whole file with the parser's rescans, returning the token count. Errors
/// skip to the next line (concatenated inputs can confuse the regex heuristic).
pub fn lex_all(src: &[u8]) -> usize {
    let mut d = Driver::new(src).expect("lexer input");
    let mut n = 0;
    loop {
        match d.next() {
            Ok(t) if t.kind == TokenKind::Eof => return n,
            Ok(_) => n += 1,
            Err(e) => {
                let mut p = e.pos as usize + 1;
                while p < src.len() && src[p] != b'\n' {
                    p += 1;
                }
                d.lx = Lexer::new(&src[p..]).expect("lexer input");
                d.templates.clear();
            }
        }
    }
}

/// The token-level driver from djs-core's lexer tests: decides regex vs division and
/// handles template continuations like the parser will.
struct Driver<'a> {
    lx: Lexer<'a>,
    prev: Option<Token>,
    parens: Vec<bool>,
    last_paren_was_head: bool,
    templates: Vec<usize>,
    depth: usize,
}

impl<'a> Driver<'a> {
    fn new(src: &'a [u8]) -> Result<Self, LexError> {
        Ok(Driver {
            lx: Lexer::new(src)?,
            prev: None,
            parens: Vec::new(),
            last_paren_was_head: false,
            templates: Vec::new(),
            depth: 0,
        })
    }

    fn regex_allowed(&self) -> bool {
        use TokenKind::*;
        let Some(prev) = self.prev else { return true };
        match prev.kind {
            Ident => matches!(prev.kw, Kw::Yield | Kw::Await) && !prev.has_escape(),
            RParen => self.last_paren_was_head,
            Number | BigInt | String | NoSubstTemplate | TemplateTail | Regex | PrivateName
            | RBracket | This | Super | True | False | Null => false,
            PlusPlus | MinusMinus => false,
            _ => true,
        }
    }

    fn next(&mut self) -> Result<Token, LexError> {
        use TokenKind::*;
        let mut tok = self.lx.next_token()?;
        match tok.kind {
            Slash | SlashAssign if self.regex_allowed() => tok = self.lx.rescan_as_regex(tok)?,
            Gt => tok = self.lx.rescan_greater(tok),
            LBrace => self.depth += 1,
            RBrace if self.templates.last() == Some(&self.depth) => {
                tok = self.lx.rescan_template_continuation(tok)?;
                if tok.kind == TemplateTail {
                    self.templates.pop();
                }
            }
            RBrace => self.depth = self.depth.saturating_sub(1),
            TemplateHead => self.templates.push(self.depth),
            LParen => {
                let head = self
                    .prev
                    .is_some_and(|p| matches!(p.kind, If | While | For | With));
                self.parens.push(head);
            }
            RParen => self.last_paren_was_head = self.parens.pop().unwrap_or(false),
            _ => {}
        }
        self.prev = Some(tok);
        Ok(tok)
    }
}

// ---- number conversion ------------------------------------------------------------------

/// A small deterministic PRNG (xorshift64*), as in djs-core's tests.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

pub fn num(r: &mut Runner) {
    let mut rng = Rng(42);
    let n = r.scale(1000, 50);
    let ints: Vec<f64> = (0..n).map(|_| rng.below(1_000_000) as f64).collect();
    let short: Vec<f64> = (0..n).map(|_| rng.below(100_000) as f64 / 100.0).collect();
    let random: Vec<f64> = (0..n)
        .map(|_| f64::from_bits(rng.next() >> 2).abs())
        .filter(|x| x.is_finite())
        .collect();
    let each = |xs: &[f64], f: &dyn Fn(f64)| {
        for &x in xs {
            f(black_box(x));
        }
    };
    r.ops("num/to_string(int)", ints.len(), || {
        each(&ints, &|x| {
            black_box(to_string(x));
        })
    });
    r.ops("num/to_string(short decimal)", short.len(), || {
        each(&short, &|x| {
            black_box(to_string(x));
        })
    });
    r.ops("num/to_string(random bits)", random.len(), || {
        each(&random, &|x| {
            black_box(to_string(x));
        })
    });
    r.ops("num/to_fixed(short decimal, 2)", short.len(), || {
        each(&short, &|x| {
            black_box(to_fixed(x, 2).unwrap());
        })
    });
    r.ops("num/to_precision(random bits, 6)", random.len(), || {
        each(&random, &|x| {
            black_box(to_precision(x, Some(6)).unwrap());
        })
    });
    r.ops("num/to_string_radix(random bits, 16)", random.len(), || {
        each(&random, &|x| {
            black_box(to_string_radix(x, 16).unwrap());
        })
    });

    let strs = |xs: &[f64]| {
        xs.iter()
            .map(|&x| to_string(x).as_str().to_string())
            .collect::<Vec<_>>()
    };
    let (int_s, short_s, random_s) = (strs(&ints), strs(&short), strs(&random));
    let long_s: Vec<String> = random.iter().map(|&x| format!("{x:.30e}")).collect();
    let wide: Vec<Vec<u16>> = random_s
        .iter()
        .map(|s| s.encode_utf16().collect())
        .collect();
    let parse_each = |xs: &[String]| {
        for s in xs {
            black_box(string_to_number(black_box(s.as_bytes())));
        }
    };
    r.ops("num/string_to_number(int)", int_s.len(), || {
        parse_each(&int_s)
    });
    r.ops("num/string_to_number(short decimal)", short_s.len(), || {
        parse_each(&short_s)
    });
    r.ops(
        "num/string_to_number(shortest of random)",
        random_s.len(),
        || parse_each(&random_s),
    );
    r.ops("num/string_to_number(31 digits)", long_s.len(), || {
        parse_each(&long_s)
    });
    r.ops("num/string_to_number(u16, shortest)", wide.len(), || {
        for s in &wide {
            black_box(string_to_number(black_box(&s[..])));
        }
    });
}

// ---- regexp --------------------------------------------------------------------------------

fn u16s(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

pub fn regexp(r: &mut Runner) {
    let compile_patterns = [
        ("\\d+", ""),
        ("[\\w.+-]+@[\\w-]+\\.[\\w.]+", ""),
        ("https?://([^/\\s]+)(/[^\\s?#]*)?", ""),
        ("(?<y>\\d{4})-(?<m>\\d{2})-(?<d>\\d{2})", ""),
        ("\\bfox\\b", "i"),
        ("^\\s+|\\s+$", "g"),
    ];
    let reps = r.scale(1000, 10);
    for (p, f) in compile_patterns {
        let pat = u16s(p);
        let flags = Flags::parse(&u16s(f)).unwrap();
        r.ops(&format!("regexp/compile /{p}/{f}"), reps, || {
            for _ in 0..reps {
                black_box(compile(black_box(&pat), flags).unwrap());
            }
        });
    }
    let text = "The quick brown fox jumps over the lazy dog. Contact john.doe@example.com or visit https://example.com/a/b on 2024-01-15. "
        .repeat(r.scale(1000, 5));
    let utf16 = u16s(&text);
    let latin1: Vec<u8> = text.bytes().collect();
    for (p, f) in [
        ("[\\w.+-]+@[\\w-]+\\.[\\w.]+", "g"),
        ("\\d+", "g"),
        ("(?<y>\\d{4})-(?<m>\\d{2})-(?<d>\\d{2})", "g"),
        ("\\bfox\\b", "gi"),
        ("lazy dog", "g"),
        ("zebra", "g"),
        ("\\s+", "g"),
    ] {
        let prog = compile(&u16s(p), Flags::parse(&u16s(f)).unwrap()).unwrap();
        for (kind, input) in [
            ("utf16", Input::Utf16(&utf16)),
            ("latin1", Input::Latin1(&latin1)),
        ] {
            r.bytes(&format!("regexp/scan /{p}/{f} {kind}"), text.len(), || {
                black_box(scan(&prog, input));
            });
        }
    }
    // Property escapes and long single-class runs (0056): multilingual words, and pi's
    // base64 check on 1 MB.
    let words = "Grüße, Привет! word 漢字 αβγ abc;def\n".repeat(r.scale(2000, 5));
    let words16 = u16s(&words);
    let prog = compile(&u16s("\\p{L}+"), Flags::parse(&u16s("gu")).unwrap()).unwrap();
    r.bytes("regexp/scan /\\p{L}+/gu utf16", words16.len() * 2, || {
        black_box(scan(&prog, Input::Utf16(&words16)));
    });
    let b64 = "QUJD".repeat(r.scale(262_144, 100)) + "==!";
    let prog = compile(&u16s("^[A-Za-z0-9+/]+={0,2}$"), Flags::empty()).unwrap();
    r.bytes("regexp/scan base64 check latin1", b64.len(), || {
        black_box(scan(&prog, Input::Latin1(b64.as_bytes())));
    });
}

/// Finds all matches like a global `exec` loop; returns the count.
fn scan(prog: &djs_core::regexp::Program, input: Input<'_>) -> usize {
    let mut start = 0;
    let mut count = 0;
    loop {
        let mut fuel = i64::MAX;
        match exec(prog, input, start, &mut fuel, ExecLimits::default()) {
            ExecResult::Match(c) => {
                let (s, e) = c.get(0).unwrap();
                start = if e == s { e + 1 } else { e };
                count += 1;
            }
            _ => return count,
        }
    }
}

// ---- json ----------------------------------------------------------------------------------

/// 0012's payloads: 10k similar records, and a long string with escapes.
pub fn json_payloads(records: usize, string_reps: usize) -> [(&'static str, String); 2] {
    let rows: Vec<String> = (0..records)
        .map(|i| {
            format!(
                "{{\"id\":{i},\"name\":\"item {i}\",\"path\":\"/home/user/project/src/file_{i}.ts\",\
                 \"size\":{},\"score\":{},\"tags\":[\"alpha\",\"beta\"],\"active\":{}}}",
                i * 37,
                i as f64 / 7.0,
                i % 2 == 0
            )
        })
        .collect();
    let big = format!(
        "\"{}\"",
        "lorem ipsum dolor sit amet, \\\"quoted\\\" \\n line ".repeat(string_reps)
    );
    [
        ("records", format!("[{}]", rows.join(","))),
        ("string", big),
    ]
}

pub fn json(r: &mut Runner) {
    let payloads = json_payloads(r.scale(10_000, 50), r.scale(1 << 15, 20));
    for (name, text) in &payloads {
        let mut h = Heap::new().unwrap();
        let s = h.alloc_str(text).unwrap();
        let hs = h.new_handle(Value::string(s)).unwrap();
        // Collect (untimed) before every sample, so each parse starts from the same heap.
        let h = std::cell::RefCell::new(h);
        let collect = || {
            h.borrow_mut().collect().unwrap();
        };
        r.bytes_with_setup(&format!("json/parse {name}"), text.len(), collect, || {
            let mut h = h.borrow_mut();
            let s = h.get_handle(hs).as_string().unwrap();
            black_box(parse(&mut h, s, &ParseOptions::default()).unwrap());
        });
        r.bytes_with_setup(
            &format!("json/parse_utf8 {name}"),
            text.len(),
            collect,
            || {
                black_box(
                    parse_utf8(
                        &mut h.borrow_mut(),
                        text.as_bytes(),
                        &ParseOptions::default(),
                    )
                    .unwrap(),
                );
            },
        );
        let hv = {
            let mut h = h.borrow_mut();
            let s = h.get_handle(hs).as_string().unwrap();
            let v = parse(&mut h, s, &ParseOptions::default()).unwrap();
            h.new_handle(v).unwrap()
        };
        r.bytes_with_setup(
            &format!("json/stringify {name}"),
            text.len(),
            collect,
            || {
                let mut h = h.borrow_mut();
                let v = h.get_handle(hv);
                black_box(stringify_fast(&mut h, v, &[]).unwrap().unwrap());
            },
        );
    }
}

// ---- gc ------------------------------------------------------------------------------------

pub fn gc(r: &mut Runner) {
    if r.wants("gc/records") {
        let mut h = Heap::new().unwrap();
        let root = h.new_array(None, 0).unwrap();
        let hr = h.new_handle(Value::object(root)).unwrap();
        let keys: Vec<Atom> = ["id", "name", "path", "size", "tags"]
            .iter()
            .map(|k| h.atom_str(k).unwrap())
            .collect();
        for i in 0..r.scale(200_000, 1000) as u32 {
            let o = h.new_object(None).unwrap();
            for (j, &k) in keys.iter().enumerate() {
                let v = if j % 2 == 0 {
                    Value::int(i as i32)
                } else {
                    Value::string(h.alloc_str(&format!("value {i} {j}")).unwrap())
                };
                h.create_data_property(o, k, v).unwrap();
            }
            let root = h.get_handle(hr).as_object().unwrap();
            h.set_element_fast(root, i, Value::object(o)).unwrap();
        }
        collect_bench(r, "gc/records", &mut h);
    }
    if r.wants("gc/linked list") {
        let mut h = Heap::new().unwrap();
        let k = h.atom_str("next").unwrap();
        let head = h.new_handle(Value::NULL).unwrap();
        let piece = h.alloc_str(&"x".repeat(40)).unwrap();
        let hp = h.new_handle(Value::string(piece)).unwrap();
        for _ in 0..r.scale(500_000, 1000) {
            let o = h.new_object(None).unwrap();
            let next = h.get_handle(head);
            h.create_data_property(o, k, next).unwrap();
            let p = h.get_handle(hp).as_string().unwrap();
            let rope = h.concat(p, p).unwrap();
            h.create_data_property(o, atoms::VALUE, Value::string(rope))
                .unwrap();
            h.set_handle(head, Value::object(o));
        }
        collect_bench(r, "gc/linked list", &mut h);
    }
}

/// A full collection of the fixed workload `name` built: `{name} collection` in ms, the
/// number to compare across versions, and `{name} per MiB live` from the same samples.
/// The per-MiB value alone misleads when cells get smaller: the same objects in fewer
/// bytes collect in about the same time, so it rose by 60% while collections got faster
/// (0224).
fn collect_bench(r: &mut Runner, name: &str, h: &mut Heap) {
    h.collect().unwrap();
    let mib = h.arena().len() as f64 / (1 << 20) as f64;
    let per_mib = format!("{name} per MiB live");
    r.time_and_per(
        &format!("{name} collection"),
        (&per_mib, Unit::MsPerMiB, mib),
        || {
            h.collect().unwrap();
        },
    );
}

// ---- wire ----------------------------------------------------------------------------------

/// The boundary datasets (0018/0021) natively, in ms:
/// - `wire/decode`: the inbound wire format;
/// - `wire/parse_utf8`: the same data as JSON, the inbound alternative;
/// - `wire/stringify`: `stringify_fast` of the decoded value, which the VM would do without
///   the direct-read reader (the reader itself is host code; see the wasm group).
pub fn wire(r: &mut Runner) {
    use djs_core::wire::{DecodeOptions, decode};
    if !r.wants_group("wire/") {
        return;
    }
    for (name, data) in crate::data::datasets(r.config.quick) {
        let wire = data.to_wire();
        let json = data.to_json();
        drop(data);
        let h = std::cell::RefCell::new(Heap::new().unwrap());
        // Collect (untimed) before every sample, so each decode starts from the same heap.
        let collect = || {
            h.borrow_mut().collect().unwrap();
        };
        r.time_with_setup(&format!("wire/decode {name}"), collect, || {
            black_box(decode(&mut h.borrow_mut(), &wire, &DecodeOptions::default()).unwrap());
        });
        r.time_with_setup(&format!("wire/parse_utf8 {name}"), collect, || {
            let v = parse_utf8(
                &mut h.borrow_mut(),
                json.as_bytes(),
                &ParseOptions::default(),
            );
            black_box(v.unwrap());
        });
        let hv = {
            let mut h = h.borrow_mut();
            let v = decode(&mut h, &wire, &DecodeOptions::default()).unwrap();
            h.new_handle(v).unwrap()
        };
        r.time_with_setup(&format!("wire/stringify {name}"), collect, || {
            let mut h = h.borrow_mut();
            let v = h.get_handle(hv);
            black_box(stringify_fast(&mut h, v, &[]).unwrap().unwrap());
        });
    }
}

// ---- startup -------------------------------------------------------------------------------

/// Creating a VM (0032): a copy of the base snapshot against building the realm and
/// compiling the prelude, plus the size of the base (it ships in every binary).
pub fn startup(r: &mut Runner) {
    use djs_core::vm::{Vm, VmOptions};
    let n = r.scale(200, 2);
    r.ops("startup/vm_new from base", n, || {
        for _ in 0..n {
            black_box(djs_prelude::new_vm(VmOptions::default()).unwrap());
        }
    });
    let n = r.scale(20, 1);
    r.ops("startup/vm_new compiling the prelude", n, || {
        for _ in 0..n {
            black_box(Vm::new(VmOptions::default()).unwrap());
        }
    });
    r.value(
        "startup/base snapshot size",
        crate::runner::Unit::Bytes,
        djs_prelude::BASE.len() as f64,
    );
}
