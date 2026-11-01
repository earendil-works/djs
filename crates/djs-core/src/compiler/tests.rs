//! Compiler tests: disassembly snapshots (`snapshots/*.js` → `*.disasm`) and errors.
//!
//! Regenerate the snapshots with `DJS_UPDATE_SNAPSHOTS=1 cargo test -p djs-core
//! compiler` and review the diff.

extern crate std;

use alloc::string::String;
use alloc::vec::Vec;
use std::path::PathBuf;

use super::*;
use crate::bytecode::template::disasm_template;
use crate::heap::Heap;

const BODY_PARAMS: &[&str] = &["tools", "console"];

fn compile_src(heap: &mut Heap, src: &str) -> Result<Ref, CompileError> {
    let opts = if src.starts_with("// goal: body") {
        CompileOptions::async_body("test.js", BODY_PARAMS)
    } else {
        CompileOptions::script("test.js")
    };
    compile(heap, src.as_bytes(), &opts)
}

#[test]
fn snapshots() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/compiler/snapshots");
    let update = std::env::var_os("DJS_UPDATE_SNAPSHOTS").is_some();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "js"))
        .collect();
    files.sort();
    assert!(!files.is_empty());
    let mut failed = Vec::new();
    for js in files {
        let src = std::fs::read_to_string(&js).unwrap();
        let mut heap = Heap::new().unwrap();
        let text = match compile_src(&mut heap, &src) {
            Ok(t) => disasm_template(&heap, t),
            Err(e) => alloc::format!("error: {e}\n"),
        };
        let expected_path = js.with_extension("disasm");
        if update {
            std::fs::write(&expected_path, &text).unwrap();
            continue;
        }
        let expected = std::fs::read_to_string(&expected_path).unwrap_or_default();
        if expected != text {
            failed.push(alloc::format!(
                "{}:\n--- expected\n{expected}\n--- got\n{text}",
                js.display()
            ));
        }
    }
    assert!(
        failed.is_empty(),
        "snapshot mismatch (DJS_UPDATE_SNAPSHOTS=1 to update):\n{}",
        failed.join("\n")
    );
}

fn err(src: &str) -> CompileError {
    let mut heap = Heap::new().unwrap();
    match compile_src(&mut heap, src) {
        Ok(_) => panic!("{src:?} compiled"),
        Err(e) => e,
    }
}

/// `(source, message, line, column)`; messages and positions follow qjs where it has
/// one (checked with `qjs` on `"use strict";` + source, column shifted by 13).
#[test]
fn errors() {
    let cases: &[(&str, &str, u32, u32)] = &[
        (
            "1 +",
            "unexpected token in expression: 'end of input'",
            1,
            4,
        ),
        (
            "let x = 1; let x = 2;",
            "invalid redefinition of lexical identifier",
            1,
            18,
        ),
        (
            "var x; let x;",
            "invalid redefinition of global identifier",
            1,
            13,
        ),
        (
            "// goal: body\nvar x; let x;",
            "invalid redefinition of a variable",
            2,
            13,
        ),
        (
            "let x; { var x; }",
            "invalid redefinition of lexical identifier",
            1,
            15,
        ),
        (
            "{ let x; var x; }",
            "invalid redefinition of lexical identifier",
            1,
            15,
        ),
        (
            "let a, a;",
            "invalid redefinition of lexical identifier",
            1,
            9,
        ),
        ("const x;", "missing initializer for const variable", 1, 8),
        ("1 = 2", "invalid assignment left-hand side", 1, 3),
        ("a + 1 = 2", "invalid assignment left-hand side", 1, 7),
        ("({a}) = 1", "invalid assignment left-hand side", 1, 7),
        ("1++", "invalid increment/decrement operand", 1, 2),
        ("++f()", "invalid increment/decrement operand", 1, 6),
        (
            "delete x",
            "cannot delete a direct reference in strict mode",
            1,
            1,
        ),
        ("var eval", "invalid variable name in strict mode", 1, 5),
        ("eval = 1", "invalid lvalue in strict mode", 1, 1),
        ("arguments++", "invalid lvalue in strict mode", 1, 1),
        ("var let", "variable name expected", 1, 5),
        ("var yield", "variable name expected", 1, 5),
        ("var \\u0069f", "variable name expected", 1, 5),
        ("let let = 1", "variable name expected", 1, 5),
        ("var if", "variable name expected", 1, 5),
        ("yield = 1", "unexpected 'yield' keyword", 1, 1),
        (
            "x = l\\u0065t",
            "unexpected token in expression: 'let'",
            1,
            5,
        ),
        (
            "-a ** 2",
            "unparenthesized unary expression can't appear on the left-hand side of '**'",
            1,
            4,
        ),
        ("a ?? b || c", "cannot mix ?? with && or ||", 1, 3),
        ("a || b ?? c", "cannot mix ?? with && or ||", 1, 8),
        ("throw\n1", "line terminator not allowed after throw", 2, 1),
        ("return 1", "return not in a function", 1, 1),
        ("break", "break must be inside loop or switch", 1, 1),
        ("continue", "continue must be inside loop", 1, 1),
        ("a: { continue a; }", "break/continue label not found", 1, 6),
        (
            "while (1) { break b; }",
            "break/continue label not found",
            1,
            13,
        ),
        ("a: a: 1", "duplicate label name", 1, 4),
        (
            "if (1) let x = 1",
            "lexical declarations can't appear in single-statement context",
            1,
            8,
        ),
        (
            "while (1) const x = 1;",
            "lexical declarations can't appear in single-statement context",
            1,
            11,
        ),
        (
            "new.target",
            "new.target only allowed within functions",
            1,
            1,
        ),
        ("super.x", "'super' is only valid in a method", 1, 1),
        ("with (a) {}", "invalid keyword: with", 1, 1),
        ("export var x", "unsupported keyword: export", 1, 1),
        (
            "switch (1) { default: default: }",
            "duplicate default",
            1,
            23,
        ),
        ("x\n=>1", "unexpected token in expression: '=>'", 2, 1),
        ("a.#x", "private names are not supported yet", 1, 3),
        ("`${`", "unexpected end of string", 1, 4),
        ("`\\x`", "malformed escape sequence in string literal", 1, 2),
        ("let x = 1 2", "expecting ';'", 1, 11),
        ("f(1, 2", "expecting ')'", 1, 7),
        ("({a = 1})", "expecting '}'", 1, 5),
        (
            "({a = 1} = b)",
            "destructuring assignment is not supported yet",
            1,
            3,
        ),
        ("({1n: 1})", "BigInt literals are not supported yet", 1, 3),
        (
            "'\\08'",
            "octal escape sequences are not allowed in strict mode",
            1,
            2,
        ),
        (
            "x = 08",
            "octal literals are deprecated in strict mode",
            1,
            5,
        ),
        ("a?.b = 1", "invalid assignment left-hand side", 1, 6),
        (
            "new a?.b()",
            "new keyword cannot be used with an optional chain",
            1,
            6,
        ),
        // Not supported yet in P0.
        ("function f() {}", "functions are not supported yet", 1, 1),
        ("x => 1", "arrow functions are not supported yet", 1, 1),
        ("(a, b) => 1", "arrow functions are not supported yet", 1, 1),
        (
            "async function f() {}",
            "async functions are not supported yet",
            1,
            1,
        ),
        (
            "async x => 1",
            "async arrow functions are not supported yet",
            1,
            1,
        ),
        ("class A {}", "classes are not supported yet", 1, 1),
        ("let [a] = b", "destructuring is not supported yet", 1, 5),
        (
            "[a, b] = c",
            "destructuring assignment is not supported yet",
            1,
            8,
        ),
        (
            "for (const x of y) {}",
            "for-in and for-of loops are not supported yet",
            1,
            1,
        ),
        (
            "for (x in y) {}",
            "for-in and for-of loops are not supported yet",
            1,
            1,
        ),
        (
            "try {} catch {}",
            "try statements are not supported yet",
            1,
            1,
        ),
        ("x = /a/", "regular expressions are not supported yet", 1, 5),
        ("f(...a)", "spread arguments are not supported yet", 1, 3),
        ("f`x`", "tagged templates are not supported yet", 1, 2),
        ("({ m() {} })", "methods are not supported yet", 1, 4),
        (
            "({ get x() {} })",
            "getters, setters and methods are not supported yet",
            1,
            4,
        ),
        ("1n", "BigInt literals are not supported yet", 1, 1),
        ("// goal: body\nawait x", "await is not supported yet", 2, 1),
    ];
    let mut bad = Vec::new();
    for &(src, msg, line, col) in cases {
        let e = err(src);
        if (e.message.as_str(), e.line, e.col) != (msg, line, col)
            || e.kind != CompileErrorKind::Syntax
        {
            bad.push(alloc::format!(
                "{src:?}: got {:?} at {}:{}, want {msg:?} at {line}:{col}",
                e.message,
                e.line,
                e.col
            ));
        }
    }
    assert!(bad.is_empty(), "\n{}", bad.join("\n"));
}

#[test]
fn display() {
    let e = err("1 +");
    assert_eq!(
        alloc::format!("{e}"),
        "SyntaxError: unexpected token in expression: 'end of input' at 1:4"
    );
    let _: String = e.message;
}

/// Nested constructs, `n` levels deep.
fn nested(kind: &str, n: usize) -> String {
    let (open, mid, close) = match kind {
        "paren" => ("(", "1", ")"),
        "array" => ("[", "1", "]"),
        "object" => ("{a:", "1", "}"),
        "unary" => ("!", "x", ""),
        "ternary" => ("a ? b : ", "c", ""),
        "block" => ("{", "", "}"),
        "if" => ("if (a) ", "x;", ""),
        "assign" => ("a = ", "1", ""),
        "member" => ("", "a", ".b"),
        "new" => ("new ", "X", ""),
        _ => unreachable!(),
    };
    let mut s = String::from("x = ");
    if matches!(kind, "block" | "if") {
        s.clear();
    }
    for _ in 0..n {
        s.push_str(open);
    }
    s.push_str(mid);
    for _ in 0..n {
        s.push_str(close);
    }
    s
}

const KINDS: &[&str] = &[
    "paren", "array", "object", "unary", "ternary", "block", "if", "assign", "member", "new",
];

/// The deepest nesting of each construct that compiles: run on a thread with the wasm
/// default stack (1 MiB; debug builds get more, their frames are larger), so hitting
/// the depth limit is a SyntaxError, never a stack overflow (docs/compiler.md 2.4).
#[test]
fn depth_limit_fits_wasm_stack() {
    let stack = match std::env::var("DJS_DEPTH_STACK") {
        // For measuring the headroom: DJS_DEPTH_STACK=262144 cargo test --release ...
        Ok(s) => s.parse().unwrap(),
        Err(_) if cfg!(debug_assertions) => 8 << 20,
        Err(_) => 1 << 20,
    };
    std::thread::Builder::new()
        .stack_size(stack)
        .spawn(|| {
            for kind in KINDS {
                // Find the largest n that compiles, by doubling then bisecting.
                let ok = |n: usize| {
                    let mut heap = Heap::new().unwrap();
                    compile(
                        &mut heap,
                        nested(kind, n).as_bytes(),
                        &CompileOptions::script("t"),
                    )
                    .is_ok()
                };
                let (mut lo, mut hi) = (1, 2);
                while ok(hi) && hi < 1 << 13 {
                    lo = hi;
                    hi *= 2;
                }
                while hi - lo > 1 {
                    let m = (lo + hi) / 2;
                    if ok(m) {
                        lo = m;
                    } else {
                        hi = m;
                    }
                }
                std::println!("{kind}: max depth {lo}");
                // Chains that loop rather than recurse have no limit.
                if *kind == "member" {
                    assert!(lo >= 1 << 12, "{kind}: {lo}");
                    continue;
                }
                assert!(lo >= 300, "{kind}: {lo}");
                let mut heap = Heap::new().unwrap();
                let e = compile(
                    &mut heap,
                    nested(kind, hi).as_bytes(),
                    &CompileOptions::script("t"),
                )
                .unwrap_err();
                assert_eq!(e.message, "stack overflow", "{kind}");
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

fn test262_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("DJS_TEST262") {
        return Some(dir.into());
    }
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()?;
    let common = PathBuf::from(String::from_utf8(out.stdout).ok()?.trim());
    let dir = common.parent()?.join("vendor/test262/test");
    dir.is_dir().then_some(dir)
}

fn collect_js(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_js(&path, out);
        } else if path.extension().is_some_and(|e| e == "js") {
            out.push(path);
        }
    }
}

/// Compiles every test262 test (without its harness). Must not panic. Lists the
/// non-negative tests that fail with something other than "not supported yet":
/// candidates for parser bugs. `cargo test -p djs-core --release compile_test262 --
/// --ignored --nocapture`
#[test]
#[ignore]
fn compile_test262() {
    let Some(root) = test262_dir() else {
        std::println!("vendor/test262 not found; skipping");
        return;
    };
    let mut files = Vec::new();
    collect_js(&root, &mut files);
    files.sort();
    let (mut ok, mut unsupported, mut negative, mut sloppy) = (0, 0, 0, 0);
    let mut other: Vec<String> = Vec::new();
    // Negative parse tests that compile: missing early errors.
    let mut missed: Vec<String> = Vec::new();
    for path in &files {
        let src = std::fs::read(path).unwrap();
        let text = String::from_utf8_lossy(&src);
        let mut heap = Heap::new().unwrap();
        let r = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
            compile(&mut heap, &src, &CompileOptions::script("t.js"))
        }));
        let Ok(r) = r else {
            panic!("compiler panicked on {}", path.display());
        };
        match r {
            Ok(_) if text.contains("phase: parse") => {
                let rel = path.strip_prefix(&root).unwrap_or(path);
                missed.push(alloc::format!("{}", rel.display()));
            }
            Ok(_) => ok += 1,
            Err(e) if e.message.ends_with("not supported yet") => unsupported += 1,
            Err(_) if text.contains("negative:") => negative += 1,
            Err(_) if text.contains("noStrict") || text.contains("flags: [raw]") => sloppy += 1,
            // Modules are not supported (TESTING.md skips them).
            Err(_) if text.contains("module]") || text.contains(", module") => sloppy += 1,
            Err(e) => {
                let rel = path.strip_prefix(&root).unwrap_or(path);
                other.push(alloc::format!(
                    "{}:{}:{}: {}",
                    rel.display(),
                    e.line,
                    e.col,
                    e.message
                ));
            }
        }
    }
    std::println!(
        "{} files: {ok} ok, {unsupported} not supported yet, {negative} negative, {sloppy} sloppy or module, {} other errors",
        files.len(),
        other.len()
    );
    for o in &other {
        std::println!("  {o}");
    }
    std::println!(
        "{} negative parse tests compile (missing early errors):",
        missed.len()
    );
    for m in &missed {
        std::println!("  missed {m}");
    }
}
