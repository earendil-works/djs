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
