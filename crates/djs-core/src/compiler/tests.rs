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
        // Functions (0024).
        (
            "function f(a, a) {}",
            "duplicate argument names not allowed in this context",
            1,
            16,
        ),
        (
            "(a, a) => 1",
            "duplicate argument names not allowed in this context",
            1,
            6,
        ),
        (
            "function f(a = 1) { \"use strict\" }",
            "\"use strict\" not allowed in function with default or destructuring parameter",
            1,
            21,
        ),
        (
            "function eval() {}",
            "invalid function name in strict code",
            1,
            10,
        ),
        (
            "function f(eval) {}",
            "invalid argument name in strict code",
            1,
            12,
        ),
        (
            "(arguments) => 1",
            "invalid argument name in strict code",
            1,
            2,
        ),
        ("function f(...a, b) {}", "expecting ')'", 1, 16),
        ("function f(...a = 1) {}", "expecting ')'", 1, 17),
        ("function f(,) {}", "missing formal parameter", 1, 12),
        (
            "() => new.target",
            "new.target only allowed within functions",
            1,
            7,
        ),
        (
            "function f() { let a; var a }",
            "invalid redefinition of lexical identifier",
            1,
            29,
        ),
        (
            "function f(a) { let a }",
            "invalid redefinition of parameter name",
            1,
            23,
        ),
        (
            "{ function f(){} let f }",
            "invalid redefinition of lexical identifier",
            1,
            24,
        ),
        (
            "{ let f; function f(){} }",
            "invalid redefinition of lexical identifier",
            1,
            19,
        ),
        (
            "if (1) function f(){}",
            "function declarations can't appear in single-statement context",
            1,
            8,
        ),
        (
            "label: function f(){}",
            "function declarations can't appear in single-statement context",
            1,
            8,
        ),
        ("function () {}", "function name expected", 1, 10),
        ("function* g() {}", "generators are not supported yet", 1, 1),
        (
            "let f = function* () {}",
            "generators are not supported yet",
            1,
            9,
        ),
        (
            "function f() { x\n=> 1 }",
            "unexpected token in expression: '=>'",
            2,
            1,
        ),
        // Exceptions and lexical scoping (0025).
        ("try {}", "expecting catch or finally", 1, 7),
        ("try { throw 1 }", "expecting catch or finally", 1, 16),
        ("try {} catch () {}", "identifier expected", 1, 15),
        (
            "try {} catch (eval) {}",
            "invalid variable name in strict mode",
            1,
            15,
        ),
        ("try {} catch (e, f) {}", "expecting ')'", 1, 16),
        (
            "try {} catch (e) { let e }",
            "invalid redefinition of lexical identifier",
            1,
            26,
        ),
        (
            "try {} catch (e) { function e(){} }",
            "invalid redefinition of lexical identifier",
            1,
            29,
        ),
        (
            "try {} catch ([e]) {}",
            "destructuring catch parameters are not supported yet",
            1,
            15,
        ),
        (
            "switch (1) { case 1: let a; case 2: let a; }",
            "invalid redefinition of lexical identifier",
            1,
            42,
        ),
        (
            "function f(){} let f",
            "invalid redefinition of global identifier",
            1,
            21,
        ),
        (
            "{ function f(){} function f(){} }",
            "invalid redefinition of lexical identifier",
            1,
            27,
        ),
        (
            "for (let i;;) { var i }",
            "invalid redefinition of lexical identifier",
            1,
            23,
        ),
        (
            "try { break } finally {}",
            "break must be inside loop or switch",
            1,
            7,
        ),
        // Not supported yet.
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
        // for-in/of (0029); messages and positions as qjs.
        (
            "for (x) ;",
            "expected 'of' or 'in' in for control expression",
            1,
            7,
        ),
        (
            "for (a + 1 in {}) ;",
            "expected 'of' or 'in' in for control expression",
            1,
            8,
        ),
        (
            "for (x++ of []) ;",
            "expected 'of' or 'in' in for control expression",
            1,
            7,
        ),
        (
            "for (let x, y of []) ;",
            "expected 'of' or 'in' in for control expression",
            1,
            11,
        ),
        (
            "for (let x = 1 in {}) ;",
            "a declaration in the head of a for-in loop can't have an initializer",
            1,
            16,
        ),
        (
            "for (var x = 1 of []) ;",
            "a declaration in the head of a for-of loop can't have an initializer",
            1,
            16,
        ),
        (
            "for (a = 1 of []) ;",
            "a declaration in the head of a for-of loop can't have an initializer",
            1,
            12,
        ),
        ("for (x of a, b) ;", "expecting ')'", 1, 12),
        ("for (x of y", "expecting ')'", 1, 12),
        ("for (x of ;", "expecting ';'", 1, 8),
        (
            "for (async of []) ;",
            "'for of' expression cannot start with 'async'",
            1,
            6,
        ),
        (
            "for (a?.b of []) ;",
            "invalid for in/of left hand-side",
            1,
            11,
        ),
        (
            "for (this of []) ;",
            "invalid for in/of left hand-side",
            1,
            11,
        ),
        (
            "for (new A of []) ;",
            "invalid for in/of left hand-side",
            1,
            12,
        ),
        (
            "for (const x of []) const y = 1;",
            "lexical declarations can't appear in single-statement context",
            1,
            21,
        ),
        (
            "for (let x of []) { var x; }",
            "invalid redefinition of lexical identifier",
            1,
            26,
        ),
        (
            "let x; for (var x of []) ;",
            "invalid redefinition of lexical identifier",
            1,
            19,
        ),
        (
            "for (let [a] of []) ;",
            "destructuring is not supported yet",
            1,
            10,
        ),
        (
            "for ([a] of []) ;",
            "destructuring assignment is not supported yet",
            1,
            10,
        ),
        (
            "for await (x of y) ;",
            "for await loops are not supported yet",
            1,
            5,
        ),
        ("x = /a/", "regular expressions are not supported yet", 1, 5),
        ("f`x`", "tagged templates are not supported yet", 1, 2),
        // Objects (0027).
        (
            "({ get x(a) {} })",
            "invalid number of arguments for getter or setter",
            1,
            11,
        ),
        (
            "({ set x() {} })",
            "invalid number of arguments for getter or setter",
            1,
            10,
        ),
        (
            "({ set x(...a) {} })",
            "invalid number of arguments for getter or setter",
            1,
            14,
        ),
        (
            "({ __proto__: 1, __proto__: 2 })",
            "duplicate __proto__ property name",
            1,
            31,
        ),
        (
            "({ '__proto__': 1, __proto__: 2 })",
            "duplicate __proto__ property name",
            1,
            33,
        ),
        ("({ m() { super.x } })", "super is not supported yet", 1, 10),
        (
            "({ async m() {} })",
            "async methods are not supported yet",
            1,
            4,
        ),
        ("({ *g() {} })", "generators are not supported yet", 1, 4),
        ("({ #x: 1 })", "invalid property name", 1, 4),
        ("({ get #x() {} })", "invalid property name", 1, 8),
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

// ---------------------------------------------------------------------------------
// End to end: compiled code on the interpreter (0015), in async-body mode with a host
// `console.log`. Expected outputs were checked against qjs.

/// Runs `src` as a codemode body with `console` as its parameter. Returns the printed
/// lines and the completion (`=> value` or `throw error`).
fn run_body(src: &str) -> String {
    use crate::vm::{Status, Vm, VmOptions};
    const LOG: u32 = 1;
    let mut vm = Vm::new(VmOptions::default()).unwrap();
    let console = vm.new_object().unwrap();
    let log = vm.new_host_function(LOG, "log").unwrap();
    vm.define(console, "log", log).unwrap();
    let opts = CompileOptions::async_body("e2e.js", &["console"]);
    let template = match compile(vm.heap_mut(), src.as_bytes(), &opts) {
        Ok(t) => t,
        Err(e) => return alloc::format!("compile error: {e}"),
    };
    let main = vm.instantiate(template).unwrap();
    let this = crate::value::Value::object(vm.global());
    vm.call(main, this, &[crate::value::Value::object(console)])
        .unwrap();
    let mut out = String::new();
    loop {
        match vm.run(1_000_000) {
            Status::Done(v) => {
                let s = vm.display(v);
                out.push_str(&alloc::format!("=> {s}"));
                return out;
            }
            Status::Threw(e) => {
                let s = vm.display(e);
                out.push_str(&alloc::format!("throw {s}"));
                return out;
            }
            Status::HostCall { call_id, fn_id } => {
                assert_eq!(fn_id, LOG);
                let info = vm.host_call(call_id).unwrap();
                let parts: Vec<String> = info.args.iter().map(|&a| vm.display(a)).collect();
                out.push_str(&parts.join(" "));
                out.push('\n');
                vm.resolve_host_call(call_id, Ok(crate::value::Value::UNDEFINED))
                    .unwrap();
            }
            Status::OutOfFuel => return out + "out of fuel",
            Status::Idle => return out + "idle",
        }
    }
}

#[test]
fn end_to_end() {
    let cases: &[(&str, &str)] = &[
        ("return 1 + 2 * 3", "=> 7"),
        ("let s = 'a'; s += 'b' + 1; return s", "=> ab1"),
        (
            "let x = 0; for (let i = 0; i < 10; i++) { if (i % 2) continue; x += i } return x",
            "=> 20",
        ),
        (
            "let n = 0; while (true) { n++; if (n > 4) break } return n",
            "=> 5",
        ),
        ("let n = 10; do { n -= 3 } while (n > 0); return n", "=> -2"),
        (
            "let i = 0; for (;;) { if (++i == 3) break } return i",
            "=> 3",
        ),
        ("let j = 0; for (; j < 3;) j++; return j", "=> 3"),
        (
            "let r = ''; outer: for (let i = 0; i < 3; i++) { for (let j = 0; j < 3; j++) { if (j == 2) continue outer; if (i == 2) break outer; r += `${i}${j},` } } return r",
            "=> 00,01,10,11,",
        ),
        (
            "let out = ''; for (let v = 0; v < 5; v++) { switch (v) { case 0: out += 'zero '; case 1: out += 'one '; break; default: out += 'd' + v + ' '; break; case 3: out += 'three ' } } return out",
            "=> zero one one d2 three d4 ",
        ),
        (
            "let o = {a: 1, b: {c: 2}}; o.a += 10; o.b.c++; return o.a + o.b.c",
            "=> 14",
        ),
        (
            "let a = [1, 2, 3]; a[1] = 20; return a.length + a[1]",
            "=> 23",
        ),
        ("let o = null; return o?.x ?? 'none'", "=> none"),
        ("let o = {f: {g: 5}}; return o?.f?.g", "=> 5"),
        ("let x; x ??= 4; x ||= 5; x &&= x + 1; return x", "=> 5"),
        (
            "return typeof undeclared + ' ' + typeof 1 + ' ' + typeof 'a'",
            "=> undefined number string",
        ),
        ("let t = `a${1 + 1}b${'c'}`; return t", "=> a2bc"),
        (
            "console.log('hello', 1, true); console.log(); return",
            "hello 1 true\n\n=> undefined",
        ),
        (
            "let x = 1; { let x = 2; console.log(x) } return x",
            "2\n=> 1",
        ),
        ("return 2 ** 3 ** 2", "=> 512"),
        ("return (1, 2, 3)", "=> 3"),
        ("return 5 > 3 ? 'yes' : 'no'", "=> yes"),
        ("let a = 5; return -a + +'3' + ~1 + !a", "=> -4"),
        ("return 'x' in {x: 1}", "=> true"),
        ("x; let x = 1", "throw ReferenceError: x is not initialized"),
        ("const c = 1; c = 2", "throw TypeError: 'c' is read-only"),
        ("return nope", "throw ReferenceError: 'nope' is not defined"),
        ("throw 'boom'", "throw boom"),
        ("let n = 0; n++; n--; ++n; return n", "=> 1"),
        ("let a = 7; return a >> 1 | a << 2 & 0xff ^ 3", "=> 31"),
        ("return 0.1 + 0.2", "=> 0.30000000000000004"),
        ("return 1_000_000 + 0x10 + 0b11 + 0o7", "=> 1000026"),
    ];
    let mut bad = Vec::new();
    for &(src, want) in cases {
        let got = run_body(src);
        if got != want {
            bad.push(alloc::format!("{src}\n  got:  {got:?}\n  want: {want:?}"));
        }
    }
    assert!(bad.is_empty(), "\n{}", bad.join("\n"));
}

/// Functions and closures end to end (docs/compiler.md 5.3 to 5.9). Outputs checked
/// against qjs (`work/scratch/0024/qjs_check.py`).
#[test]
fn end_to_end_functions() {
    run_cases(&[
        (
            "function counter() { let n = 0; return () => ++n } const c = counter(); c(); c(); return c()",
            "=> 3",
        ),
        (
            "const fs = []; for (let i = 0; i < 3; i++) fs[i] = () => i; return fs[0]() + fs[1]() * 10 + fs[2]() * 100",
            "=> 210",
        ),
        (
            "var fs = []; for (var i = 0; i < 3; i++) fs[i] = () => i; return fs[0]()",
            "=> 3",
        ),
        (
            "const fs = []; for (let i = 0; i < 3; i++) { let j = i * 2; fs[i] = () => j } return fs[2]()",
            "=> 4",
        ),
        (
            "function a(x) { return function (y) { return (z) => x + y + z } } return a(1)(2)(3)",
            "=> 6",
        ),
        (
            "function fact(n) { return n <= 1 ? 1 : n * fact(n - 1) } return fact(10)",
            "=> 3628800",
        ),
        (
            "return isEven(10); function isEven(n) { return n == 0 ? true : isOdd(n - 1) } function isOdd(n) { return n == 0 ? false : isEven(n - 1) }",
            "=> true",
        ),
        (
            "function f(a, b = a * 2) { return a + b } return f(3) + f(3, 1)",
            "=> 13",
        ),
        (
            "function f(a = b, b) {} f()",
            "throw ReferenceError: b is not initialized",
        ),
        (
            "function f(x) { const g = () => x; x = 5; return g() } return f(1)",
            "=> 5",
        ),
        (
            "const c = 1; const f = () => { c = 2 }; f()",
            "throw TypeError: 'c' is read-only",
        ),
        (
            "{ function inner() { return 9 } } return typeof inner",
            "=> undefined",
        ),
        ("{ function inner() { return 9 } return inner() }", "=> 9"),
        (
            "const f = () => x; f(); let x = 1",
            "throw ReferenceError: x is not initialized",
        ),
        ("const add = (a, b) => a + b; return add(2, 3)", "=> 5"),
        (
            "let x = 1; function setX(v) { x = v } setX(5); return x",
            "=> 5",
        ),
        (
            "function mk() { let a = 0; return { inc: () => ++a, get: () => a } } const m = mk(); m.inc(); m.inc(); return m.get()",
            "=> 2",
        ),
    ]);
}

/// The function cases that need 0022's interpreter pieces (`special_object`,
/// `construct`, function `name`/`length`/`prototype`, `rest`). Un-ignore when 0022
/// lands. Checked against qjs too.
#[test]
fn end_to_end_functions_0022() {
    run_cases(&[
        (
            "const o = { v: 42, m: function () { return this.v } }; return o.m()",
            "=> 42",
        ),
        (
            "const o = { v: 7, m: function () { const f = () => this.v + arguments[0]; return f(100) } }; return o.m(1)",
            "=> 8",
        ),
        (
            "function foo(a, b = 1, c) {} return foo.name + foo.length",
            "=> foo1",
        ),
        (
            "const bar = () => 1; let baz; baz = function () {}; return bar.name + baz.name",
            "=> barbaz",
        ),
        ("function P(x) { this.x = x } return new P(4).x", "=> 4"),
        (
            "function F() { return new.target } return F()",
            "=> undefined",
        ),
        (
            "function F() { this.t = new.target === F } return new F().t",
            "=> true",
        ),
        (
            "function f() { return arguments.length } return f(1, 2, 3)",
            "=> 3",
        ),
        (
            "function f(a, ...r) { return a + r.length } return f(1, 2, 3)",
            "=> 3",
        ),
        ("function f(...r) { return r.length } return f()", "=> 0"),
        (
            "const f = function g(n) { return n ? g(n - 1) + 1 : 0 }; return f(3)",
            "=> 3",
        ),
    ]);
}

fn run_cases(cases: &[(&str, &str)]) {
    let mut bad = Vec::new();
    for &(src, want) in cases {
        let got = run_body(src);
        if got != want {
            bad.push(alloc::format!("{src}\n  got:  {got:?}\n  want: {want:?}"));
        }
    }
    assert!(bad.is_empty(), "\n{}", bad.join("\n"));
}

/// Exceptions end to end (docs/compiler.md 7.4). Outputs checked against qjs
/// (`work/scratch/0025/qjs_check.py end_to_end_exceptions`).
#[test]
fn end_to_end_exceptions() {
    run_cases(&[
        ("try { throw 1 } catch (e) { return e + 1 }", "=> 2"),
        ("try { throw 'x' } catch { return 'caught' }", "=> caught"),
        (
            "let log = ''; try { log += 'a' } finally { log += 'b' } return log",
            "=> ab",
        ),
        (
            "let log = ''; try { throw 1 } catch (e) { log += 'c' + e } finally { log += 'f' } return log",
            "=> c1f",
        ),
        (
            "function f() { try { return 'try' } finally { console.log('fin') } } return f()",
            "fin\n=> try",
        ),
        (
            "function f() { try { return 1 } finally { return 2 } } return f()",
            "=> 2",
        ),
        (
            "function f() { try { throw 1 } finally { return 3 } } return f()",
            "=> 3",
        ),
        (
            "let log = ''; for (let i = 0; i < 4; i++) { try { if (i == 1) continue; if (i == 3) break; log += i } finally { log += 'f' } } return log",
            "=> 0ff2ff",
        ),
        (
            "let log = ''; outer: for (let i = 0; i < 3; i++) { for (;;) { try { try { continue outer } finally { log += 'a' } } finally { log += 'b' + i } } } return log",
            "=> ab0ab1ab2",
        ),
        (
            "function f() { let log = ''; try { try { return log += 'r' } finally { log += 'x' } } finally { console.log(log) } } return f()",
            "rx\n=> r",
        ),
        (
            "let n = 0; while (true) { try { break } finally { n++ } } return n",
            "=> 1",
        ),
        (
            "let n = 0; while (true) { try { continue } finally { n++; if (n > 2) break } } return n",
            "=> 3",
        ),
        (
            "L: try { break L } finally { console.log('f') } return 'after'",
            "f\n=> after",
        ),
        (
            "try { try { throw 1 } finally { console.log('inner') } } catch (e) { return 'outer ' + e }",
            "inner\n=> outer 1",
        ),
        (
            "try { try { throw 1 } catch (e) { throw e + 1 } } catch (e) { return e }",
            "=> 2",
        ),
        (
            "try { try { throw 1 } finally { throw 2 } } catch (e) { return e }",
            "=> 2",
        ),
        (
            "function f() { throw 'deep' } function g() { return f() } try { g() } catch (e) { return e }",
            "=> deep",
        ),
        (
            "try { null.x } catch (e) { return e.message }",
            "=> cannot read property 'x' of null",
        ),
        (
            "try { undefinedName } catch (e) { return e.name + ': ' + e.message }",
            "=> ReferenceError: 'undefinedName' is not defined",
        ),
        (
            "let e = 'outer'; try { throw 'inner' } catch (e) { } return e",
            "=> outer",
        ),
        ("try { throw 1 } catch (e) { var v = e } return v", "=> 1"),
        (
            "const fs = []; for (let i = 0; i < 2; i++) { try { throw i } catch (e) { fs[i] = () => e } } return fs[0]() + fs[1]()",
            "=> 1",
        ),
        (
            "function f() { try { return 'a' } catch (e) { return 'b' } finally { } } return f()",
            "=> a",
        ),
        ("let x = 0; try { x = 1 } catch { x = 2 } return x", "=> 1"),
        (
            "switch (1) { case 1: try { break } finally { console.log('f') } } return 'ok'",
            "f\n=> ok",
        ),
        ("throw 'top'", "throw top"),
    ]);
}

/// Lexical scoping edge cases (P1-4). Outputs checked against qjs.
#[test]
fn end_to_end_lexical() {
    run_cases(&[
        (
            "typeof x; let x = 1",
            "throw ReferenceError: x is not initialized",
        ),
        (
            "let x = 1; { x; let x = 2 }",
            "throw ReferenceError: x is not initialized",
        ),
        ("let x = 1; { let x = 2; return x }", "=> 2"),
        (
            "switch (1) { case 0: let y = 1; case 1: return y }",
            "throw ReferenceError: y is not initialized",
        ),
        ("switch (0) { case 0: let y = 1; case 1: return y }", "=> 1"),
        ("const f = () => z; let z = 3; return f()", "=> 3"),
        (
            "function g() { return w } g(); let w",
            "throw ReferenceError: w is not initialized",
        ),
        (
            "let a = 1; { function a2() { return a } let a = 2; return a2() }",
            "=> 2",
        ),
        ("let x = 'outer'; { let x = 'inner' } return x", "=> outer"),
        ("const c = 1; { const c = 2 } return c", "=> 1"),
        (
            "let s = 0; for (let i = 0; i < 3; i++) { let i = 10; s += i } return s",
            "=> 30",
        ),
        (
            "let r = []; for (let i = 0; i < 3; i++) { r[i] = () => i; i += 0 } return r[0]() + r[2]()",
            "=> 2",
        ),
        ("{ let q = 1 } return typeof q", "=> undefined"),
        ("let fn; { let k = 5; fn = () => k } return fn()", "=> 5"),
    ]);
}

/// The generated prelude (D1172) compiles in privileged mode.
#[test]
fn prelude_compiles() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../builtins/gen/prelude.js");
    let src = std::fs::read(&path).unwrap();
    let mut heap = Heap::new().unwrap();
    let opts = CompileOptions::privileged_script("prelude.js");
    let t = compile(&mut heap, &src, &opts).unwrap_or_else(|e| panic!("{e}"));
    let text = disasm_template(&heap, t);
    assert!(text.contains("call_intrinsic %"), "{text}");
    // Without the flag, `%` is just an operator.
    let e = compile(&mut heap, &src, &CompileOptions::script("prelude.js")).unwrap_err();
    assert_eq!(e.kind, CompileErrorKind::Syntax);
}

#[test]
fn privileged_errors() {
    let cases: &[(&str, &str)] = &[
        ("%Nope(1)", "unknown intrinsic %Nope"),
        ("%IsObject()", "wrong number of arguments for %IsObject"),
        ("%IsObject(1, 2)", "wrong number of arguments for %IsObject"),
        ("%Call()", "wrong number of arguments for %Call"),
        ("%IsObject(...a)", "spread arguments in an intrinsic call"),
        ("% 1", "intrinsic name expected"),
    ];
    for &(src, msg) in cases {
        let mut heap = Heap::new().unwrap();
        let e = compile(
            &mut heap,
            src.as_bytes(),
            &CompileOptions::privileged_script("p.js"),
        )
        .unwrap_err();
        assert_eq!(e.message, msg, "{src}");
    }
    let mut heap = Heap::new().unwrap();
    let ok = "%Call(f, undefined); %Call(f, undefined, 1, 2); %IsObject(1)";
    let t = compile(
        &mut heap,
        ok.as_bytes(),
        &CompileOptions::privileged_script("p.js"),
    );
    let text = disasm_template(&heap, t.unwrap());
    assert!(text.contains("call_intrinsic %Call 4"), "{text}");
    let e = compile(&mut heap, b"%IsObject(1)", &CompileOptions::script("p.js")).unwrap_err();
    assert_eq!(e.message, "unexpected token in expression: '%'");
}

/// Objects end to end (P1-2). Outputs checked against qjs
/// (`work/scratch/0027/qjs_check.py end_to_end_objects`).
#[test]
fn end_to_end_objects() {
    run_cases(&[
        (
            "const o = { a: 1, 'b c': 2, 3: 3 }; return o.a + o['b c'] + o[3]",
            "=> 6",
        ),
        (
            "const k = 'x'; const o = { [k + 'y']: 5 }; return o.xy",
            "=> 5",
        ),
        (
            "const a = 1, b = 2; const o = { a, b }; return o.a + o.b",
            "=> 3",
        ),
        ("const o = { a: 1 }; delete o.a; return o.a", "=> undefined"),
        ("const o = { a: 1 }; return delete o.a", "=> true"),
        (
            "const o = { a: 1 }; const k = 'a'; delete o[k]; return 'a' in o",
            "=> false",
        ),
        ("let o = null; return delete o?.a", "=> true"),
        (
            "let o = { a: { b: 1 } }; delete o?.a.b; return o.a.b",
            "=> undefined",
        ),
        (
            "const o = { a: 1 }; return 'a' in o && !('b' in o)",
            "=> true",
        ),
        (
            "function F() {} const f = new F(); return f instanceof F",
            "=> true",
        ),
        ("const o = { f: () => 1 }; return o.f.name", "=> f"),
    ]);
}

/// Object cases that need 0028 (method/accessor definitions, `set_proto`,
/// `set_function_name`, spread calls). Un-ignore when 0028 lands. Checked against qjs.
#[test]
fn end_to_end_objects_0028() {
    run_cases(&[
        (
            "const o = { v: 2, m(x) { return this.v * x } }; return o.m(5)",
            "=> 10",
        ),
        ("const o = { get g() { return 7 } }; return o.g", "=> 7"),
        (
            "let s = 0; const o = { set v(x) { s = x } }; o.v = 9; return s",
            "=> 9",
        ),
        (
            "const o = { _v: 1, get v() { return this._v }, set v(x) { this._v = x * 2 } }; o.v = 4; return o.v",
            "=> 8",
        ),
        (
            "const k = 'dyn'; const o = { [k]() {} }; return o.dyn.name",
            "=> dyn",
        ),
        (
            "const k = 'p'; const o = { get [k]() { return 1 } }; return o.p",
            "=> 1",
        ),
        (
            "const k = 'nm'; const o = { [k]: () => 1 }; return o.nm.name",
            "=> nm",
        ),
        ("const o = { m() {} }; return o.m.name", "=> m"),
        (
            "const p = { x: 1 }; const o = { __proto__: p }; return o.x",
            "=> 1",
        ),
        (
            "const o = { __proto__: null }; return 'toString' in o",
            "=> false",
        ),
        (
            "const o = { ...{ a: 1, b: 2 }, c: 3 }; return o.a + o.b + o.c",
            "=> 6",
        ),
        (
            "function f(a, b, c) { return a + b + c } const args = [1, 2]; return f(...args, 3)",
            "=> 6",
        ),
        (
            "const o = { m(...r) { return r.length } }; return o.m(...[1, 2], ...[3])",
            "=> 3",
        ),
        (
            "function P(a, b) { this.s = a + b } return new P(...[1, 2]).s",
            "=> 3",
        ),
    ]);
}

/// The helper of the iteration cases: `it(vals, ret)` is an iterable over `vals` that
/// logs `n` for each `next` call and `r` for each `return` call (`ret` replaces
/// `return`; `null` removes it).
const ITER_PREFIX: &str = "const log = []; function it(vals, ret) { return { [Symbol.iterator]() { let i = 0; return { next() { log.push('n'); return i < vals.length ? { value: vals[i++], done: false } : { value: undefined, done: true }; }, return: ret === undefined ? function () { log.push('r'); return {}; } : ret }; } }; }\n";

fn run_cases_with(prefix: &str, cases: &[(&str, &str)]) {
    let mut bad = Vec::new();
    for &(src, want) in cases {
        let got = run_body(&alloc::format!("{prefix}{src}"));
        if got != want {
            bad.push(alloc::format!("{src}\n  got:  {got:?}\n  want: {want:?}"));
        }
    }
    assert!(bad.is_empty(), "\n{}", bad.join("\n"));
}

/// for-of and for-in end to end (docs/compiler.md 3.8, 7.4; D1290, D1292). Outputs
/// checked against qjs (`work/scratch/0029/qjs_check.py`).
#[test]
fn end_to_end_iteration() {
    run_cases_with(
        ITER_PREFIX,
        &[
            (
                "let s = 0; for (const x of it([1, 2, 3])) s += x; return s + ' ' + log.join('')",
                "=> 6 nnnn",
            ),
            ("for (var v of it([4, 5])) ; return v", "=> 5"),
            ("let a; for (a of it([1, 2])) ; return a", "=> 2"),
            ("const o = {}; for (o.p of it([7, 8])) ; return o.p", "=> 8"),
            (
                "const o = {}; let k = 0; for (o[k++] of it(['a', 'b'])) ; return o[0] + o[1] + k",
                "=> ab2",
            ),
            (
                "for (const x of it([1, 2, 3])) { if (x == 2) break; } return log.join('')",
                "=> nnr",
            ),
            (
                "for (const x of it([1, 2, 3])) { if (x < 3) continue; console.log(x); } return log.join('')",
                "3\n=> nnnn",
            ),
            (
                "outer: for (const x of it([1, 2])) { for (const y of it([1, 2])) { continue outer; } } return log.join('')",
                "=> nnrnnrn",
            ),
            (
                "outer: for (const x of it([1, 2])) { for (const y of it([1, 2])) { break outer; } } return log.join('')",
                "=> nnrr",
            ),
            (
                "L: { for (const x of it([1, 2])) break L; } return log.join('')",
                "=> nr",
            ),
            (
                "function f() { for (const x of it([1, 2])) { return x + 10; } } return f() + ' ' + log.join('')",
                "=> 11 nr",
            ),
            (
                "function f() { for (const x of it([1])) { for (const y of it([2])) return x + y; } } return f() + ' ' + log.join('')",
                "=> 3 nnrr",
            ),
            (
                "function f() { for (const x of it([1])) { return; } } return f() + ' ' + log.join('')",
                "=> undefined nr",
            ),
            (
                "try { for (const x of it([1, 2])) throw new Error('boom'); } catch (e) { return e.message + ' ' + log.join(''); }",
                "=> boom nr",
            ),
            (
                "try { for (const x of it([1, 2], function () { throw new Error('ret'); })) throw new Error('body'); } catch (e) { return e.message; }",
                "=> body",
            ),
            (
                "try { for (const x of it([1, 2], function () { throw new Error('ret'); })) break; } catch (e) { return e.message; }",
                "=> ret",
            ),
            (
                "try { for (const x of it([1, 2], function () { return 5; })) break; } catch (e) { return String(e); }",
                "=> TypeError: not an object",
            ),
            (
                "try { for (const x of it([1, 2], function () { return 5; })) throw 'body'; } catch (e) { return String(e); }",
                "=> body",
            ),
            (
                "for (const x of it([1, 2], null)) break; return log.join('')",
                "=> n",
            ),
            (
                "function f() { for (const x of it([1, 2], function () { throw 'r'; })) return 1; } try { f(); } catch (e) { return e; }",
                "=> r",
            ),
            (
                "for (const x of it([1, 2])) { switch (x) { case 1: break; } } return log.join('')",
                "=> nnn",
            ),
            (
                "for (const x of it([1, 2])) { try { break; } finally { log.push('f'); } } return log.join('')",
                "=> nfr",
            ),
            (
                "function f() { try { for (const x of it([1])) return 'v'; } finally { log.push('f'); } } return f() + log.join('')",
                "=> vnrf",
            ),
            (
                "for (const x of it([1, 2])) { try { throw 1; } catch (e) { log.push('c'); } } return log.join('')",
                "=> ncncn",
            ),
            (
                "try { for (const x of it([1, 2], function () { log.push('r'); throw 'r'; })) { try { break; } finally { log.push('f'); } } } catch (e) { log.push('c' + e); } return log.join('')",
                "=> nfrcr",
            ),
            (
                "const o = {}; Object.defineProperty(o, 'p', { set(v) { if (v == 2) throw 'set'; } }); try { for (o.p of it([1, 2, 3])) ; } catch (e) { return e + log.join(''); }",
                "=> setnnr",
            ),
            (
                "try { for (const x of { [Symbol.iterator]() { return { next() { return 1; }, return() { log.push('r'); } }; } }) ; } catch (e) { return String(e) + log.join(''); }",
                "=> TypeError: iterator must return an object",
            ),
            (
                "try { for (const x of { [Symbol.iterator]() { return { next() { throw 'next'; }, return() { log.push('r'); } }; } }) ; } catch (e) { return e + log.join(''); }",
                "=> next",
            ),
            (
                "try { for (const x of { [Symbol.iterator]() { return { next() { return { get done() { throw 'done'; } }; }, return() { log.push('r'); } }; } }) ; } catch (e) { return e + log.join(''); }",
                "=> done",
            ),
            (
                "try { for (const x of { [Symbol.iterator]() { return { next() { return { done: false, get value() { throw 'value'; } }; }, return() { log.push('r'); } }; } }) ; } catch (e) { return e + log.join(''); }",
                "=> value",
            ),
            (
                "try { for (const x of 1) ; } catch (e) { return String(e); }",
                "=> TypeError: value is not iterable",
            ),
            (
                "try { for (const x of {}) ; } catch (e) { return String(e); }",
                "=> TypeError: value is not iterable",
            ),
            (
                "try { for (const x of { [Symbol.iterator]: 5 }) ; } catch (e) { return String(e); }",
                "=> TypeError: value is not iterable",
            ),
            (
                "try { for (const x of { [Symbol.iterator]() { return 1; } }) ; } catch (e) { return String(e); }",
                "=> TypeError: not an object",
            ),
            (
                "try { for (const x of { [Symbol.iterator]() { return {}; } }) ; } catch (e) { return String(e); }",
                "=> TypeError: not a function",
            ),
            (
                "let n = 0; const iter = { [Symbol.iterator]() { return this; }, next() { n++; this.next = null; return { done: n > 2, value: n }; } }; let s = ''; for (const x of iter) s += x; return s",
                "=> 12",
            ),
            (
                "const fs = []; for (const x of it([1, 2, 3])) fs.push(() => x); return fs.map((f) => f()).join()",
                "=> 1,2,3",
            ),
            (
                "const fs = []; for (let x of it([1, 2, 3])) { fs.push(() => x); x *= 10; } return fs.map((f) => f()).join()",
                "=> 10,20,30",
            ),
            (
                "const fs = []; for (var x of it([1, 2, 3])) fs.push(() => x); return fs.map((f) => f()).join()",
                "=> 3,3,3",
            ),
            (
                "try { for (let x of x) ; } catch (e) { return String(e); }",
                "=> ReferenceError: x is not initialized",
            ),
            (
                "try { for (const x of it([1])) x = 2; } catch (e) { return String(e) + log.join(''); }",
                "=> TypeError: 'x' is read-onlynr",
            ),
            (
                "let x = 'outer'; for (let x of it(['inner'])) { } return x",
                "=> outer",
            ),
            (
                "let r = ''; for (const x of it(['a'])) { let x2 = x + 'b'; { let x = 'c'; r += x2 + x; } } return r",
                "=> abc",
            ),
            (
                "const ks = []; const o = { b: 1, a: 2, 2: 3, 1: 4 }; for (const k in o) ks.push(k); return ks.join()",
                "=> 1,2,b,a",
            ),
            (
                "const ks = []; const p = { x: 1, y: 2 }; const o = Object.create(p); o.z = 3; o.x = 4; for (const k in o) ks.push(k); return ks.join()",
                "=> z,x,y",
            ),
            (
                "const ks = []; const o = { a: 1 }; Object.defineProperty(o, 'h', { value: 1, enumerable: false }); for (const k in o) ks.push(k); return ks.join()",
                "=> a",
            ),
            (
                "const ks = []; const p = { s: 1 }; const o = Object.create(p); Object.defineProperty(o, 's', { value: 1, enumerable: false }); for (const k in o) ks.push(k); return ks.length",
                "=> 0",
            ),
            (
                "const ks = []; const o = { a: 1, b: 2, c: 3 }; for (const k in o) { ks.push(k); delete o.b; o.d = 4; } return ks.join()",
                "=> a,c",
            ),
            (
                "const ks = []; const o = { [Symbol('s')]: 1, a: 1 }; for (const k in o) ks.push(typeof k); return ks.join()",
                "=> string",
            ),
            (
                "let n = 0; for (const k in null) n++; for (const k in undefined) n++; return n",
                "=> 0",
            ),
            (
                "const ks = []; for (const k in 'ab') ks.push(k); return ks.join()",
                "=> 0,1",
            ),
            (
                "const ks = []; for (const k in 5) ks.push(k); return ks.length",
                "=> 0",
            ),
            (
                "const ks = []; for (const k in [7, 8]) ks.push(typeof k + k); return ks.join()",
                "=> string0,string1",
            ),
            (
                "const o = {}; for (o.k in { a: 1, b: 2 }) ; return o.k",
                "=> b",
            ),
            ("var k; for (k in { a: 1 }) ; return k", "=> a"),
            (
                "let s = ''; for (const k in { a: 1, b: 2, c: 3 }) { if (k == 'b') continue; if (k == 'c') break; s += k; } return s",
                "=> a",
            ),
            (
                "let s = ''; outer: for (const k in { a: 1, b: 1 }) { for (const j in { x: 1, y: 1 }) { s += k + j; continue outer; } } return s",
                "=> axbx",
            ),
            (
                "function f() { for (const k in { a: 1 }) { for (const x of it([1])) return k + x; } } return f() + log.join('')",
                "=> a1nr",
            ),
            (
                "const fs = []; for (const k in { a: 1, b: 2 }) fs.push(() => k); return fs.map((f) => f()).join()",
                "=> a,b",
            ),
            (
                "try { for (let k in k) ; } catch (e) { return String(e); }",
                "=> ReferenceError: k is not initialized",
            ),
            (
                "let s = ''; for (const k in { a: 1 }) for (const j in { b: 1 }) s += k + j; return s",
                "=> ab",
            ),
            (
                "let i = 0; for (const x of it([1, 2, 3])) { i++; } return i + log.join('')",
                "=> 3nnnn",
            ),
        ],
    );
}

/// Iteration over arrays (the Array Iterator leaves of 0030, D1290). Outputs checked
/// against qjs.
#[test]
fn end_to_end_iteration_rt() {
    run_cases(&[
        (
            "let s = 0; for (const x of [1, 2, 3]) s += x; return s",
            "=> 6",
        ),
        (
            "let s = ''; for (const x of [1, , 3]) s += x; return s",
            "=> 1undefined3",
        ),
        (
            "const a = [1, 2]; let s = ''; for (const x of a) { s += x; if (a.length < 4) a.push(a.length + 1); } return s",
            "=> 1234",
        ),
        (
            "const a = [1, 2, 3]; let s = ''; for (const x of a) { s += x; a.length = 1; } return s",
            "=> 1",
        ),
        (
            "let s = ''; for (const e of ['a', 'b'].entries()) s += e[0] + e[1]; return s",
            "=> 0a1b",
        ),
        (
            "let s = ''; for (const k of ['a', 'b'].keys()) s += k; return s",
            "=> 01",
        ),
        (
            "let s = ''; for (const v of ['a', 'b'].values()) s += v; return s",
            "=> ab",
        ),
        (
            "const it = [1, 2][Symbol.iterator](); return it[Symbol.iterator]() === it",
            "=> true",
        ),
        (
            "const it = [1][Symbol.iterator](); const r1 = it.next(); const r2 = it.next(); return r1.value + ' ' + r1.done + ' ' + r2.value + ' ' + r2.done",
            "=> 1 false undefined true",
        ),
        (
            "try { [].values().next.call({}); } catch (e) { return String(e); }",
            "=> TypeError: Array Iterator object expected",
        ),
        (
            "let s = ''; for (const x of { length: 2, 0: 'a', 1: 'b', [Symbol.iterator]: Array.prototype.values }) s += x; return s",
            "=> ab",
        ),
        (
            "const p = Array.prototype[Symbol.iterator]; let n = 0; Array.prototype[Symbol.iterator] = function () { n++; return p.call(this); }; for (const x of [1]) ; Array.prototype[Symbol.iterator] = p; return n",
            "=> 1",
        ),
        (
            "let s = ''; for (const x of [1, 2, 3]) { if (x == 2) break; s += x; } return s",
            "=> 1",
        ),
        (
            "const fs = []; for (const x of [1, 2]) fs.push(() => x); return fs[0]() + fs[1]()",
            "=> 3",
        ),
        (
            "return Object.getPrototypeOf(Object.getPrototypeOf([].values())) === Object.getPrototypeOf(Object.getPrototypeOf([][Symbol.iterator]()))",
            "=> true",
        ),
        ("return [].values === [][Symbol.iterator]", "=> true"),
        (
            "const d = Object.getOwnPropertyDescriptor(Object.getPrototypeOf([].values()), Symbol.toStringTag); return Object.prototype.toString.call([].values()) + ' ' + d.value + d.writable + d.enumerable + d.configurable",
            "=> [object Array Iterator] Array Iteratorfalsefalsetrue",
        ),
    ]);
}

/// Messages from %GetIterator on nullish values (a symbol key in "cannot read
/// property", as qjs names it).
#[test]
fn end_to_end_iteration_messages() {
    run_cases(&[(
        "try { for (const x of undefined) ; } catch (e) { return String(e); }",
        "=> TypeError: cannot read property 'Symbol.iterator' of undefined",
    )]);
}
