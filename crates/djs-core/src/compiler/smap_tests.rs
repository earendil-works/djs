//! Suspension maps (compiler/smap.rs, D2563): roles per construct.

extern crate std;

use alloc::string::String;
use alloc::vec::Vec;

use super::{CompileOptions, suspension_maps};
use crate::bytecode::template::disasm_template;
use crate::heap::Heap;

/// The operand roles of each site of the first function with sites, as one line per
/// site (`-` for none, `!construct` for an unsupported site).
fn roles(src: &str) -> Vec<String> {
    let mut heap = Heap::new().unwrap();
    let opts = CompileOptions::async_body("t.js", &[]);
    let maps = suspension_maps(&mut heap, src.as_bytes(), &opts).unwrap();
    let mut out = Vec::new();
    for m in &maps {
        for s in &m.sites {
            out.push(match &s.unsupported {
                Some(u) => alloc::format!("{} !{u}", s.name),
                None if s.operands.is_empty() => alloc::format!("{} -", s.name),
                None => {
                    let r: Vec<String> = s.operands.iter().map(|r| alloc::format!("{r}")).collect();
                    alloc::format!("{} {}", s.name, r.join(" "))
                }
            });
        }
    }
    out
}

/// `(source, the roles of each site)` for each construct that keeps operands.
const CONSTRUCTS: &[(&str, &[&str])] = &[
    // Calls: callee and `this`, then the arguments so far.
    (
        "f(await a, await b);",
        &[
            "await#1 call.callee call.this",
            "await#2 call.callee call.this call.arg[0]",
        ],
    ),
    (
        "o.m(a, await b).n(await c);",
        &[
            "await#1 call.callee call.this call.arg[0]",
            "await#2 call.callee call.this",
        ],
    ),
    (
        "f(...xs, await y);",
        &["await#1 call.callee call.this call.args"],
    ),
    ("a?.b(await c);", &["await#1 call.callee call.this"]),
    ("f?.(await x);", &["await#1 call.callee call.this"]),
    ("(a?.b)(await x);", &["await#1 call.callee call.this"]),
    (
        "tag`a${x}b${await c}`;",
        &["await#1 call.callee call.this call.arg[0] call.arg[1]"],
    ),
    (
        "await f(await g(await h()));",
        &[
            "await#1 -",
            "await#2 call.callee call.this",
            "await#3 call.callee call.this call.callee call.this",
        ],
    ),
    ("new X(a, await b);", &["await#1 new.callee new.arg[0]"]),
    ("new X(...(await xs));", &["await#1 new.callee new.args"]),
    ("new (await X)();", &["await#1 -"]),
    ("import(a, await b);", &["await#1 import.specifier"]),
    // Members and assignments.
    ("o[await k];", &["await#1 member.object"]),
    ("o?.[await k];", &["await#1 member.object"]),
    ("delete o[await k];", &["await#1 member.object"]),
    ("o.p = await v;", &["await#1 assign.object"]),
    ("o[k] = await v;", &["await#1 assign.object assign.key"]),
    (
        "x.y = z.w = await q;",
        &["await#1 assign.object assign.object"],
    ),
    ("x += await v;", &["await#1 assign.old"]),
    ("o.p += await v;", &["await#1 assign.object assign.old"]),
    ("o[k] ??= await v;", &["await#1 assign.object assign.key"]),
    ("x = y = await z;", &["await#1 -"]),
    // Operators: only the ones that keep their left operand.
    ("a + await b;", &["await#1 binary.left"]),
    (
        "a = b + (c * await d);",
        &["await#1 binary.left binary.left"],
    ),
    ("a in await b;", &["await#1 binary.left"]),
    ("a && await b;", &["await#1 -"]),
    ("c ? await a : b;", &["await#1 -"]),
    ("(await a, await b);", &["await#1 -", "await#2 -"]),
    // Literals.
    ("[a, await b];", &["await#1 array.element[0]"]),
    ("[, await b];", &["await#1 array.literal"]),
    ("[...xs, await y];", &["await#1 array.literal"]),
    (
        "({ a: 1, [k]: await v });",
        &["await#1 object.literal object.key"],
    ),
    ("({ [await k]: 1 });", &["await#1 object.literal"]),
    ("({ a, ...(await o) });", &["await#1 object.literal"]),
    (
        "x = { y: [1, await z] };",
        &["await#1 object.literal array.element[0]"],
    ),
    (
        "`${a}: ${await b}`;",
        &["await#1 template.part[0] template.part[1]"],
    ),
    ("`${await a}`;", &["await#1 -"]),
    // Generators.
    (
        "function* h() { const v = yield 1; yield [v, yield 2]; }",
        &["yield#1 -", "yield#2 -", "yield#3 array.element[0]"],
    ),
    (
        "async function* g() { f(yield x); }",
        &[
            "yield.await#1 call.callee call.this",
            "yield#1 call.callee call.this",
            "yield.return#1 call.callee call.this",
        ],
    ),
    (
        "async function* g() { return await x; }",
        &["return.await#1 -", "await#1 -"],
    ),
];

/// The loops whose iterator records stay on the stack.
const LOOPS: &[(&str, &[&str])] = &[
    (
        "for (const x of xs) await x;",
        &["await#1 for-of.iterator for-of.next"],
    ),
    ("for (const k in o) await k;", &["await#1 for-in.iterator"]),
    (
        "for await (const x of xs) await x;",
        &[
            "for-await.close#1 exception",
            "for-await.next#1 for-await.iterator for-await.next",
            "await#1 for-await.iterator for-await.next",
        ],
    ),
    (
        "for (const x of xs) { for (const k in x) { f(await k); } }",
        &["await#1 for-of.iterator for-of.next for-in.iterator call.callee call.this"],
    ),
    // Loops whose code is rotated: the sites follow their instructions.
    (
        "for (let i = 0; i < await n; i++) { await i; }",
        &["await#1 binary.left", "await#2 -"],
    ),
    ("while (await a) { await b; }", &["await#1 -", "await#2 -"]),
    // Delegation: the inner iterator and its `next` below every suspension.
    (
        "function* h() { f(yield* xs); }",
        &["yield*#1 call.callee call.this yield*.iterator yield*.next"],
    ),
    (
        "async function* h() { yield* xs; }",
        &[
            "yield*#1 yield*.iterator yield*.next",
            "yield*.await#1 yield*.iterator yield*.next",
            "yield*.await#2 yield*.iterator yield*.next",
            "yield*.await#3 yield*.iterator yield*.next",
            "yield*.await#4 -",
            "yield*.await#5 yield*.iterator yield*.next",
            "yield*.await#6 yield*.iterator yield*.next",
            "yield*.await#7 yield*.iterator yield*.next",
        ],
    ),
    (
        "do { await b; } while (await a);",
        &["await#1 -", "await#2 -"],
    ),
];

/// Case tests and subroutines (finally blocks, a for-await's close on a jump out).
const STATEMENTS: &[(&str, &[&str])] = &[
    (
        "switch (a) { case await b: break; }",
        &["await#1 switch.discriminant switch.discriminant"],
    ),
    ("switch (a) { case 1: await b; }", &["await#1 -"]),
    (
        "try { a } finally { await b }",
        &["await#1 finally.value finally.return"],
    ),
    (
        "try { await a } catch (e) { await e }",
        &["await#1 -", "await#2 -"],
    ),
    (
        "for (const x of xs) { try { await x } finally { f(await y) } }",
        &[
            "await#1 for-of.iterator for-of.next",
            "await#2 for-of.iterator for-of.next finally.value finally.return call.callee call.this",
        ],
    ),
    (
        "try { try {} finally { await a } } finally { await b }",
        &[
            "await#1 finally.value finally.return",
            "await#2 finally.value finally.return",
        ],
    ),
    (
        "for await (const x of xs) { if (x) return 1; }",
        &[
            "for-await.close#1 exception",
            "for-await.next#1 for-await.iterator for-await.next",
            "for-await.close#2 finally.value finally.return",
        ],
    ),
];

/// Constructs whose operands are not described yet: never mislabeled.
const UNSUPPORTED: &[(&str, &[&str])] = &[
    ("const { a = await b } = o;", &["await#1 !destructuring"]),
    ("const [x] = await y;", &["await#1 -"]),
    ("class C { [await k]() {} }", &["await#1 !class"]),
    (
        "{ await using r = open(); await r.x; }",
        &["await#1 !using", "await-using#1 !using"],
    ),
];

fn check(table: &[(&str, &[&str])]) {
    let mut bad = Vec::new();
    for (src, want) in table {
        let got = roles(src);
        if got.iter().map(String::as_str).collect::<Vec<_>>() != *want {
            bad.push(alloc::format!("{src}\n  want {want:?}\n  got  {got:?}"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn operand_roles_per_construct() {
    check(CONSTRUCTS);
}

#[test]
fn loop_records() {
    check(LOOPS);
}

#[test]
fn statements() {
    check(STATEMENTS);
}

#[test]
fn unsupported_constructs() {
    check(UNSUPPORTED);
}

/// The roles come from where the compiler emits the construct: a lowering that pushed
/// `this` under the callee gives another map, which the pin test would show.
#[test]
fn a_callee_this_swap_changes_the_map() {
    let src = "f(await x);";
    assert_eq!(roles(src), ["await#1 call.callee call.this"]);
    let mut heap = Heap::new().unwrap();
    let opts = CompileOptions::async_body("t.js", &[]);
    let (_, maps) = super::suspension_maps_with(&mut heap, src.as_bytes(), &opts, |p| {
        p.test_swap_call = true;
    })
    .unwrap();
    let ops: Vec<String> = maps[0].sites[0]
        .operands
        .iter()
        .map(|r| alloc::format!("{r}"))
        .collect();
    assert_eq!(ops, ["call.this", "call.callee"]);
}

/// Every slot: bindings by name and declaring scope (nothing outside it), hidden
/// locals by name, arguments.
#[test]
fn slots_name_bindings_and_hidden_locals() {
    let src = "async function f(a, b) {\n  let x = 1;\n  { let y = 2; await a; }\n  await b;\n}";
    let mut heap = Heap::new().unwrap();
    let maps = suspension_maps(&mut heap, src.as_bytes(), &CompileOptions::script("t.js")).unwrap();
    assert_eq!(
        maps[0].text(src.as_bytes()),
        "function f @0..76\n  arg0 a@0, arg1 b@0\n  loc0 <state>, loc1 x@0, loc2 y@42\n  \
         await#1 @53 (3:16) pc 11: -\n  await#2 @66 (4:3) pc 16: - (out: loc2)\n"
    );
    let y = super::Slot::Binding {
        name: String::from("y"),
        scope: 42,
    };
    assert_eq!(maps[0].sites[0].locals[2], y);
    assert_eq!(maps[0].sites[1].locals[2], super::Slot::Nothing);
    assert_eq!(
        maps[0].sites[0].locals[0],
        super::Slot::Hidden(String::from("<state>"))
    );
}

/// A site's resume pc is right after the suspending instruction of the code a normal
/// compile produces.
#[test]
fn resume_pcs_follow_the_suspending_instruction() {
    use crate::bytecode::Op;
    use crate::bytecode::decode::insns;
    let src = "const r = [];\nfor (const x of xs) { r.push(await f(x)); }\nif (await g()) { r.push(1); }\n";
    let mut heap = Heap::new().unwrap();
    let opts = CompileOptions::async_body("t.js", &[]);
    let t = super::compile(&mut heap, src.as_bytes(), &opts).unwrap();
    let code = crate::bytecode::template::code(&heap, t.0).to_vec();
    let ends: Vec<(u32, Op)> = insns(&code)
        .map(|i| i.unwrap())
        .map(|i| (i.next_pc(), i.op))
        .collect();
    let maps = suspension_maps(&mut heap, src.as_bytes(), &opts).unwrap();
    assert_eq!(maps[0].sites.len(), 2);
    for s in &maps[0].sites {
        assert!(
            ends.contains(&(s.resume_pc, Op::Await)),
            "{} at {}",
            s.name,
            s.resume_pc
        );
    }
}

/// The pinned maps (D2563): every await and yield of the codemode corpora and the e2e
/// tests, as text next to the cases' names in tests/smap/. A change is a diff to accept
/// deliberately (`DJS_UPDATE_SMAPS=1`): dumps stopped at a changed site by an older
/// build would be refused. tests/smap/summary.txt counts the sites by role sequence.
#[test]
fn pinned_maps() {
    use std::path::{Path, PathBuf};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let update = std::env::var_os("DJS_UPDATE_SMAPS").is_some();
    // (directory, compiled as the difftest's codemode arrow?)
    let areas = [
        ("tests/corpus/codemode", "codemode", true),
        ("tests/corpus/codemode-harvest", "codemode-harvest", true),
        ("tests/data/e2e", "e2e", false),
    ];
    let mut failed = Vec::new();
    let mut seen = Vec::new();
    let mut sites = 0u32;
    let mut unsupported: std::collections::BTreeMap<String, u32> = Default::default();
    let mut sequences: std::collections::BTreeMap<String, u32> = Default::default();
    let mut kinds: std::collections::BTreeMap<String, u32> = Default::default();
    for (dir, area, corpus) in areas {
        let mut files: Vec<PathBuf> = std::fs::read_dir(root.join(dir))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "js" || e == "ts"))
            .collect();
        files.sort();
        for path in files {
            let text = std::fs::read_to_string(&path).unwrap();
            let ts = path.extension().is_some_and(|e| e == "ts");
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let (src, opts_body) = if corpus {
                // As the difftest runs a case (crates/djs-difftest/src/djs.rs).
                (
                    alloc::format!("(async (tools, console) => {{{text}\n}})"),
                    false,
                )
            } else {
                (text.clone(), text.starts_with("// mode: body"))
            };
            let legacy = text.contains("// difftest: experimental-decorators");
            let metadata = text.contains("// difftest: emit-decorator-metadata");
            let params = ["tools", "console"];
            let mut opts = if opts_body {
                CompileOptions::async_body(&name, &params)
            } else {
                CompileOptions::script(&name)
            };
            opts.typescript = ts;
            opts.experimental_decorators = legacy || metadata;
            opts.emit_decorator_metadata = metadata;
            let mut heap = Heap::new().unwrap();
            let Ok((t, maps)) =
                super::suspension_maps_with(&mut heap, src.as_bytes(), &opts, |_| {})
            else {
                continue;
            };
            // Recording changes nothing in the code.
            let mut plain = Heap::new().unwrap();
            let t2 = super::compile(&mut plain, src.as_bytes(), &opts).unwrap();
            if disasm_template(&heap, t) != disasm_template(&plain, t2) {
                failed.push(alloc::format!("{name}: recording changed the code"));
            }
            let maps: Vec<_> = maps.into_iter().filter(|m| !m.sites.is_empty()).collect();
            if maps.is_empty() {
                continue;
            }
            let mut out = String::new();
            for m in &maps {
                out.push_str(&m.text(src.as_bytes()));
                for s in &m.sites {
                    sites += 1;
                    *kinds.entry(String::from(s.kind.name())).or_default() += 1;
                    match &s.unsupported {
                        Some(u) => *unsupported.entry(u.clone()).or_default() += 1,
                        None => {
                            let seq: Vec<String> =
                                s.operands.iter().map(|r| alloc::format!("{r}")).collect();
                            let seq = if seq.is_empty() {
                                String::from("-")
                            } else {
                                seq.join(" ")
                            };
                            *sequences.entry(seq).or_default() += 1;
                        }
                    }
                }
            }
            let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
            let pin = root
                .join("tests/smap")
                .join(area)
                .join(alloc::format!("{stem}.smap"));
            seen.push(pin.clone());
            if update {
                std::fs::create_dir_all(pin.parent().unwrap()).unwrap();
                std::fs::write(&pin, &out).unwrap();
            } else if std::fs::read_to_string(&pin).unwrap_or_default() != out {
                failed.push(alloc::format!("{}", pin.display()));
            }
        }
    }
    let mut summary =
        alloc::format!("# Suspension map coverage (compiler/smap_tests.rs)\nsites {sites}\n");
    summary.push_str("\n## by kind\n");
    for (k, n) in &kinds {
        summary.push_str(&alloc::format!("{n:5} {k}\n"));
    }
    summary.push_str("\n## described, by operand roles\n");
    let mut seqs: Vec<_> = sequences.iter().collect();
    seqs.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (s, n) in seqs {
        summary.push_str(&alloc::format!("{n:5} {s}\n"));
    }
    summary.push_str("\n## unsupported\n");
    for (u, n) in &unsupported {
        summary.push_str(&alloc::format!("{n:5} {u}\n"));
    }
    let summary_path = root.join("tests/smap/summary.txt");
    if update {
        std::fs::write(&summary_path, &summary).unwrap();
        // Pins of cases that no longer have sites (or are gone).
        for area in ["codemode", "codemode-harvest", "e2e"] {
            for e in std::fs::read_dir(root.join("tests/smap").join(area)).unwrap() {
                let p = e.unwrap().path();
                if !seen.contains(&p) {
                    std::fs::remove_file(p).unwrap();
                }
            }
        }
        return;
    }
    if std::fs::read_to_string(&summary_path).unwrap_or_default() != summary {
        failed.push(alloc::format!("{}", summary_path.display()));
    }
    for area in ["codemode", "codemode-harvest", "e2e"] {
        if let Ok(dir) = std::fs::read_dir(root.join("tests/smap").join(area)) {
            for e in dir {
                let p = e.unwrap().path();
                if !seen.contains(&p) {
                    failed.push(alloc::format!("{} (stale)", p.display()));
                }
            }
        }
    }
    assert!(
        failed.is_empty(),
        "suspension maps changed (DJS_UPDATE_SMAPS=1 to accept; dumps stopped at a \
         changed site by an older build will be refused):\n{}",
        failed.join("\n")
    );
}
