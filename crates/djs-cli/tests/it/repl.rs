//! `djs repl` driven through stdin in line mode (stdin is not a terminal: no prompts, no
//! editing), with scripted inputs and the expected output.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Runs the REPL on `input` and returns its stdout. Killed after 60 s.
fn repl_with(args: &[&str], input: &str) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_djs"))
        .arg("repl")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("djs runs");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let start = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() > Duration::from_secs(60) {
            child.kill().unwrap();
            panic!("djs repl hung on:\n{input}");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "exit status {:?}", out.status);
    String::from_utf8(out.stdout).unwrap()
}

fn repl(input: &str) -> String {
    repl_with(&[], input)
}

/// [`repl`] with frozen builtins, which dumps need (D2836).
fn frozen(input: &str) -> String {
    repl_with(&["--frozen"], input)
}

#[track_caller]
fn check(input: &str, expected: &str) {
    assert_eq!(repl(input), expected, "input:\n{input}");
}

#[test]
fn expressions_print_their_value() {
    check(
        "1 + 1\n'a' + 'b'\n-0\n10n ** 20n\n[1, , 'x']\nnull\nundefined\nSymbol('s')\n",
        "2\n'ab'\n-0\n100000000000000000000n\n[ 1, <1 empty item>, 'x' ]\nnull\nundefined\nSymbol(s)\n",
    );
    // Statements print their completion value (D1581), declarations nothing.
    check(
        "if (true) { 1 }\nlet a = 1; a + 1\nlet b = 2\nfunction f() {}\nclass C {}\nvoid 0\n{a: 1}\n{ let c = 3; c }\n",
        "1\n2\nundefined\n{ a: 1 }\n3\n",
    );
}

#[test]
fn declarations_persist() {
    check(
        "let x = 2\nconst y = 3\nfunction f(a) { return a * x + y }\nclass P { constructor() { this.v = f(1) } }\nvar z = 'z'\nf(10)\nnew P()\nz\nx = 5\nf(1)\n",
        "23\nP { v: 5 }\n'z'\n5\n8\n",
    );
    // Scripts redeclare as Scripts do.
    check(
        "let x = 1\nlet x = 2\nx\n",
        "Uncaught SyntaxError: redeclaration of 'x'\n    at <eval> (repl)\n1\n",
    );
}

#[test]
fn multi_line_input() {
    check(
        "function add(a,\n  b) {\n  return a + b\n}\nadd(1,\n2)\n`one\ntwo`\n/* a\ncomment */ 7\n[\n1,\n2\n]\n",
        "3\n'one\\ntwo'\n7\n[ 1, 2 ]\n",
    );
    // An input still incomplete at the end of input is an error.
    check(
        "let a = (1 +\n",
        "SyntaxError: unexpected token in expression: '' at repl:1:13\n",
    );
}

#[test]
fn errors() {
    check(
        "throw new TypeError('bad')\nnull.x\nlet 5\n1\n",
        "Uncaught TypeError: bad\n    at <eval> (repl:1:20)\n\
         Uncaught TypeError: cannot read property 'x' of null\n    at <eval> (repl:1:5)\n\
         SyntaxError: variable name expected at repl:1:5\n1\n",
    );
    check(
        "Promise.reject(new Error('no'))\n",
        "Promise { <rejected> [Error: no] }\n\
         Possibly unhandled promise rejection: Error: no\n    at <eval> (repl:1:25)\n",
    );
}

#[test]
fn top_level_await() {
    check(
        "await Promise.resolve(1)\nlet a = await Promise.resolve(2), {b, c: [d]} = await Promise.resolve({b: 3, c: [4]})\n[a, b, d]\nasync function af() { return a + (await 10) }\nawait af()\nclass K { k = 1 }; await 0\nnew K()\nconst p = await new Promise(r => r(Promise.resolve(5)))\np\n",
        "1\n[ 2, 3, 4 ]\n12\nK { k: 1 }\n5\n",
    );
    // The value of a promise is not adopted: `await` gives what the expression does.
    check(
        "Promise.resolve(3)\nawait Promise.reject(new RangeError('r'))\n",
        "Promise { 3 }\nUncaught RangeError: r\n    at <eval> (repl:1:45)\n",
    );
    // Closures in the body see the globals, not copies.
    check(
        "let n = 0; const inc = () => ++n; await 0\ninc(), inc()\nn\n",
        "2\n2\n",
    );
}

#[test]
fn printer() {
    check(
        "let o = {a: 1, 'b-c': [1, {d: {e: {f: {}}}}], [Symbol.iterator]: 2}; o.self = o; 0\no\n",
        "0\n<ref *1> {\n  a: 1,\n  'b-c': [ 1, { d: { e: [Object] } } ],\n  self: [Circular *1],\n  [Symbol(Symbol.iterator)]: 2\n}\n",
    );
    check(
        "new Map([[1, {a: 1}]])\nnew Set(['x'])\nnew WeakMap()\nnew Uint8Array([1, 2])\nnew BigInt64Array([5n])\nnew ArrayBuffer(3)\n",
        "Map(1) { 1 => { a: 1 } }\nSet(1) { 'x' }\nWeakMap { <items unknown> }\nUint8Array(2) [ 1, 2 ]\nBigInt64Array(1) [ 5n ]\nArrayBuffer { [Uint8Contents]: <00 00 00>, byteLength: 3 }\n",
    );
    check(
        "function f() {}\nf\n(() => 1)\nasync function* g() {}\ng\nclass A {}\nclass B extends A {}\nB\nf.bind(null)\nMath.max\n",
        "[Function: f]\n[Function (anonymous)]\n[AsyncGeneratorFunction: g]\n[class B extends A]\n[Function: bound f]\n[Function: max]\n",
    );
    check(
        "new Promise(() => {})\nObject.create(null)\nnew Date(0)\n/a+/gi\n({get x() { return 1 }, set y(v) {}})\nnew Number(3)\n[new Error('e')]\n",
        "Promise { <pending> }\n[Object: null prototype] {}\n1970-01-01T00:00:00.000Z\n/a+/gi\n{ x: [Getter], y: [Setter] }\n[Number: 3]\n[ [Error: e] ]\n",
    );
    // Long values break into lines.
    check(
        "({alpha: 'aaaaaaaaaaaaaaaa', beta: 'bbbbbbbbbbbbbbbbbbbb', gamma: ['cccccccccccccccccc', 1]})\n",
        "{\n  alpha: 'aaaaaaaaaaaaaaaa',\n  beta: 'bbbbbbbbbbbbbbbbbbbb',\n  gamma: [ 'cccccccccccccccccc', 1 ]\n}\n",
    );
    check(
        "console.log('raw', 'strings', {a: 'quoted'}, [1])\n",
        "raw strings { a: 'quoted' } [ 1 ]\nundefined\n",
    );
}

#[test]
fn fuel_and_suspension() {
    check(
        "function spin(n) { let s = 0; for (let i = 0; i < n; i++) s += i; return s }\n.fuel 10000\nspin(1e9)\nspin(10)\n.fuel off\n",
        "fuel limit: 10000 per input\nStopped: the fuel limit (10000) ran out\n45\nfuel limit: none\n",
    );
    let out = repl(
        "function spin(n) { let s = 0; for (let i = 0; i < n; i++) s += i; return s }\n.suspend-every 200\nspin(30)\n",
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "suspending every 200 fuel");
    assert!(
        lines[1].starts_with("[suspended, 200 fuel] at spin (repl:1:"),
        "{out}"
    );
    assert_eq!(*lines.last().unwrap(), "435", "{out}");
    // The state survives the image round trips.
    let out = repl(
        ".suspend-every 50\nlet acc = []; for (let i = 0; i < 3; i++) acc.push({i}); 0\n.suspend-every off\nacc\n",
    );
    assert!(out.contains("\n[suspended, "), "{out}");
    assert!(
        out.ends_with("0\nsuspending: off\n[ { i: 0 }, { i: 1 }, { i: 2 } ]\n"),
        "{out}"
    );
}

#[test]
fn restore_reads_only_dumps() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let other = format!("{dir}/repl-test-{}.bin", std::process::id());
    std::fs::write(&other, b"DJS\0 not a dump").unwrap();
    let out = repl(&format!("let r = 7\n.restore {other}\nr\n"));
    assert!(out.contains("is not a djs dump"), "{out}");
    assert!(out.ends_with("\n7\n"), "{out}");
    std::fs::remove_file(&other).unwrap();
    check(
        ".restore /nonexistent/x.js\n",
        "cannot read /nonexistent/x.js: No such file or directory (os error 2)\n",
    );
}

#[test]
fn meta_commands() {
    let out = repl(".gc\n.heap\n.heap kinds\n.time on\n1\n.fuel\n");
    assert!(out.starts_with("arena: "), "{out}");
    // `.heap`: the session apart from the base realm (D2340).
    assert!(out.contains("== diff against the base image\n"), "{out}");
    assert!(out.contains("  FuncTemplate "), "{out}");
    assert!(out.contains("\ntiming: on\n1\n("), "{out}");
    assert!(out.contains("fuel)\nlast input: "), "{out}");
    let out = repl("function f(a) { return a + 1 }\n.disasm f\n.disasm let q = 1\n");
    assert!(out.starts_with("function 0 f: params=1"), "{out}");
    assert!(out.contains("init_global_lex \"q\""), "{out}");
    check(
        ".nope\n.exit\n1\n",
        "unknown command or missing argument: .nope (.help lists them)\n",
    );
    let help = repl(".help\n");
    assert!(
        help.contains(".heap [sections]") && !help.contains(".image"),
        "{help}"
    );
    check(
        ".image x.img\n",
        "unknown command or missing argument: .image x.img (.help lists them)\n",
    );
}

#[test]
fn typescript() {
    check(
        ".ts on\nlet t: number = 3\nt as number + 1\nenum E { A, B }\nE.B\nlet s: string = await Promise.resolve('ok')\ns\n",
        "TypeScript: on\n4\n1\n'ok'\n",
    );
    check(
        "let t: number = 3\n",
        "SyntaxError: expecting ';' at repl:1:6\n",
    );
    assert_eq!(repl_with(&["--ts"], "let n: number = 1\nn\n"), "1\n");
    // Top-level await over TypeScript that is not erasable (0209): an enum and a
    // parameter property keep their meaning.
    assert_eq!(
        repl_with(
            &["--ts"],
            "enum F { A, B }; await 1; console.log(F.B)\n\
             class Q { constructor(public x: number) {} }; await 1; console.log(new Q(9).x)\n"
        ),
        "1\n9\n"
    );
}

#[test]
fn load_a_file() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let path = format!("{dir}/repl-load-{}.js", std::process::id());
    std::fs::write(
        &path,
        "let loaded = await Promise.resolve(7);\nfunction g() { return loaded }\n",
    )
    .unwrap();
    let out = repl(&format!(".load {path}\ng()\n"));
    std::fs::remove_file(&path).unwrap();
    assert_eq!(out, "7\n");
}

#[test]
fn completion() {
    check(
        "let foo = {bar: 1, baz: {qux: 2}, get acc() { return {z: 1}; }}\n\
         .complete fo\n.complete foo.ba\n.complete foo.baz.q\n.complete foo.acc.\n\
         .complete Math.ma\n.complete foo.bar.toFix\nconst p = new Proxy({a: 1}, {})\n\
         .complete p.\n.complete 1.5\n.complete x = JSO\n",
        "foo\nbar baz\nqux\n\nmax\ntoFixed\n\n\nJSON\n",
    );
    // An unknown name completes nothing.
    check(".complete undefinedNam\n", "\n");
}

#[test]
fn decorators_switch() {
    let legacy = ".ts on\n.decorators legacy\n\
        function d(t: any, k: string) { t.seen = k; }\n\
        class C { @d m() {} }\n(C.prototype as any).seen\n\
        .decorators tc39\n\
        function e(v: any, ctx: any) { return v; }\n\
        class D { @e m() {} }\nD.name\n";
    assert_eq!(
        repl(legacy),
        "TypeScript: on\ndecorators: legacy (experimentalDecorators)\n'm'\ndecorators: tc39\n'D'\n"
    );
    check(
        ".decorators metadata\n.decorators x\n",
        "decorators: legacy with emitDecoratorMetadata (applies in TypeScript mode: .ts on)\n.decorators takes tc39, legacy or metadata\n",
    );
}

#[test]
fn stepping() {
    check(
        ".step 300\nfunction spin(n) { let s = 0; for (let i = 0; i < n; i++) s += i; return s }\n\
         spin(100)\n\nbt\nfuel\nhuh\nc\nspin(100)\nq\n.step off\nspin(10)\n",
        "stepping every 300 fuel\n\
         [step, 300 fuel] at spin (repl:1:64)\n\
         [step, 600 fuel] at spin (repl:1:55)\n\
         \x20   at spin (repl:1:55)\n    at <eval> (repl:1:5)\n\
         600 fuel\n\
         huh: at step> use Enter (next slice), c (continue), bt, fuel or q (stop)\n\
         4950\n\
         [step, 300 fuel] at spin (repl:1:64)\nStopped\n\
         stepping: off\n45\n",
    );
}

// ---- tools and waiting inputs (0174). Call ids are the VM's host call ids: reading a
// tool name for the first time makes the function through a host call of its own.

#[test]
fn tools_wait_and_resolve() {
    check(
        "const r = await tools.read({ path: 'a.ts' })\n.pending\nr\n.resolve 2 \"export const x = 1;\"\nr.length\n.pending\n",
        "[waiting] #2 tools.read({\"path\":\"a.ts\"})\n\
         #2 tools.read({\"path\":\"a.ts\"})  input 1, at repl:1:11\n\
         undefined\n19\nno open calls\n",
    );
    // An expression input prints its value when it settles; the prompt does not wait.
    check(
        "await tools.add(1, 2)\n'meanwhile'\n.resolve 2 3\n",
        "[waiting] #2 tools.add(1, 2)\n'meanwhile'\n[input 1] 3\n",
    );
    // A rejection, and an input waiting on calls made before it.
    check(
        "const p = tools.x()\nawait p\n.reject 2 no such thing\n.reject 2 again\n",
        "[waiting] #2 tools.x()\n[input 2 waiting]\n[input 2] Uncaught Error: no such thing\n\
         no open call #2 (.pending lists them)\n",
    );
    // Results go through the inbox: reactions run in order.
    check(
        "tools.a().then(v => console.log('a', v)); tools.b().then(v => console.log('b', v)); 0\n.resolve 4 'B'\n.resolve 4 \"B\"\n.resolve 2 [1]\n",
        "[waiting] #2 tools.a()\n[waiting] #4 tools.b()\n0\nnot JSON: SyntaxError: unexpected token: ''' at <input>:1:1\nb B\na [ 1 ]\n",
    );
}

#[test]
fn backtraces_of_waiting_activations() {
    let out = repl(
        "async function todos(path, [first], ...rest) { const body = await tools.read({path}); return body.length }\n\
         const all = Promise.all([todos('x', [1]), todos('y', [2], 3)])\n\
         await all\n.bt 3\n.bt all\n.bt 9\n.resolve 2 \"ab\"\n.resolve 3 \"cde\"\nawait all\n.bt all\n",
    );
    let expected = "[waiting] #2 tools.read({\"path\":\"x\"})\n\
        [waiting] #3 tools.read({\"path\":\"y\"})\n\
        [input 3 waiting]\n\
        #3 tools.read({\"path\":\"y\"})\n  async function todos at repl:1:61\n\
        \x20     async function todos(path, [first], ...rest) { const body = await tools.read({path}); return body.length }\n\
        \x20                                                                 ^\n\
        \x20   path = 'y', first = 2, rest = [ 3 ], body = undefined\n  async <eval> at repl:1:10 (input 3)\n";
    assert!(out.starts_with(expected), "{out}");
    assert!(
        out.contains("    path = 'x', first = 1, rest = [], body = undefined\n"),
        "{out}"
    );
    assert!(out.contains("no open call #9 (.pending lists them)\n[input 3] [ 2, 3 ]\n[ 2, 3 ]\nno suspended activations\n"), "{out}");
}

#[test]
fn answers_file() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let path = format!("{dir}/repl-answers-{}.json", std::process::id());
    std::fs::write(
        &path,
        r#"{"read": [{"args": [{"path": "a"}], "value": "A"}, {"value": {"n": 1}}],
            "fail": [{"error": "boom"}]}"#,
    )
    .unwrap();
    let out = repl_with(
        &["--tools", &path],
        "await tools.read({path: 'a'})\nawait tools.read({path: 'b'})\nawait tools.fail()\nconst p = Promise.all([tools.read({path: 'a'}), tools.glob()])\n.pending\n",
    );
    std::fs::remove_file(&path).unwrap();
    assert_eq!(
        out,
        "[answered] #2 tools.read({\"path\":\"a\"})\n'A'\n\
         [answered] #3 tools.read({\"path\":\"b\"})\n{ n: 1 }\n\
         [answered] #5 tools.fail()\nUncaught Error: boom\n\
         [answered] #6 tools.read({\"path\":\"a\"})\n[waiting] #8 tools.glob()\n\
         #8 tools.glob()  input 4\n"
    );
}

#[test]
fn restore_a_waiting_dump() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/run");
    // A hand-written dump: a run waiting on `tools.read`. Its run (the completion value)
    // becomes a waiting input; new calls of the same tool get the same function id.
    check(
        &format!(
            ".restore {dir}/reads.dump.js\n.pending\n.bt 2\n.resolve 2 \"AA\"\n.bt all\n.resolve 3 \"BBB\"\n.pending\n"
        ),
        &format!(
            "loaded {dir}/reads.dump.js into a fresh VM\n\
             waiting: #2 tools.read({{\"path\":\"a\"}})\n\
             #2 tools.read({{\"path\":\"a\"}})  the dump, at {dir}/reads.dump.js:13:12\n\
             #2 tools.read({{\"path\":\"a\"}})\n  async function <anonymous> at {dir}/reads.dump.js:13:12 (run)\n\
             \x20     const a = (await $dump.p2);\n\
             \x20                ^\n\
             \x20   a = undefined, b = undefined\n\
             [waiting] #3 tools.read({{\"path\":\"b\"}})\n\
             \x20 async function <anonymous> at {dir}/reads.dump.js:14:11 (run)\n\
             \x20     const b = await tools.read({{ path: \"b\" }});\n\
             \x20               ^\n\
             \x20   a = 'AA', b = undefined\n\
             AABBB\ntotal 5\n[run] 5\nno open calls\n"
        ),
    );
    // A plain dump (D2830) of two waiting inputs and a value: its completion value names
    // them (D2895), and the calls' states their origins (D2894).
    check(
        &format!(
            ".restore {dir}/inputs.dump.js\n.pending\n.resolve 2 \"AAA\"\n.resolve 3 \"x\"\n.pending\n"
        ),
        &format!(
            "loaded {dir}/inputs.dump.js into a fresh VM\n[done] 5\n\
             waiting: #2 tools.read({{\"path\":\"a\"}})\n\
             waiting: #3 tools.read({{\"path\":\"b\"}})\n\
             #2 tools.read({{\"path\":\"a\"}})  input 1, at {dir}/inputs.dump.js:9:60\n\
             #3 tools.read({{\"path\":\"b\"}})  input 2, at {dir}/inputs.dump.js:10:52\n\
             [input 1] 3\nno open calls\n"
        ),
    );
    // `later(v)` calls are answered at once; a program of tests/dump/v2 runs to its end.
    let tmp = env!("CARGO_TARGET_TMPDIR");
    let plan = format!("{tmp}/repl-plan-{}.js", std::process::id());
    let src = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/dump/v2/plan.src.js"
    );
    let r = Command::new(env!("CARGO_BIN_EXE_djs"))
        .args(["run", "--dump-after", "5", &plan, src])
        .output()
        .unwrap();
    assert!(r.status.success(), "{r:?}");
    let out = repl(&format!(".restore {plan}\n"));
    std::fs::remove_file(&plan).unwrap();
    assert_eq!(
        out,
        format!(
            "loaded {plan} into a fresh VM\nwaiting: #5 later(\"TODO four\\nok\\nTODO five\")\n\
             5 TODOs in 4 files\n[[2,0],[1,2]]\n[run] undefined\n"
        )
    );
    check(
        ".restore /nonexistent.js\n",
        "cannot read /nonexistent.js: No such file or directory (os error 2)\n",
    );
}

#[test]
fn dump_a_waiting_session() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let dump = format!("{dir}/repl-wait-{}.js", std::process::id());
    let out = frozen(&format!(
        "async function get(p) {{ const v = await tools.read({{path: p}}); return v.toUpperCase() }}\n\
         let out = []; get('a').then(v => out.push(v)); 0\n\
         .dump {dump}\n.clear\n.restore {dump}\n.bt all\n.resolve 2 \"hello\"\nout\n\
         .dump-every on\nget('c').then(v => out.push(v)); get('d').then(v => out.push(v)); 0\n\
         .resolve 3 \"c!\"\n.resolve 4 \"d!\"\nout\n.dump-every off\n"
    ));
    std::fs::remove_file(&dump).unwrap();
    let mut lines = out.lines();
    assert_eq!(
        lines.next(),
        Some("[waiting] #2 tools.read({\"path\":\"a\"})")
    );
    assert_eq!(lines.next(), Some("0"));
    let wrote = lines.next().unwrap();
    assert!(
        wrote.starts_with("wrote ") && wrote.ends_with(" (1 activation, 1 pending call)"),
        "{out}"
    );
    let rest: Vec<&str> = lines.collect();
    let dumped = |l: &str| l.starts_with("[dumped ") && l.ends_with(" bytes]");
    assert!(rest.iter().filter(|l| dumped(l)).count() >= 2, "{out}");
    let rest: Vec<&str> = rest.into_iter().filter(|l| !dumped(l)).collect();
    assert_eq!(
        rest,
        [
            "a fresh VM",
            &format!("loaded {dump} into a fresh VM"),
            "waiting: #2 tools.read({\"path\":\"a\"})",
            &format!("  async function <anonymous> at {dump}:9:12"),
            "      const v = (await $dump.p1);",
            "                 ^",
            // (`p` is dead after the await: the continuation leaves it out, D2911.)
            "    v = undefined",
            "[ 'HELLO' ]",
            "dumping at every idle point: on",
            // The restored tools are made already (the calls' states, D2897): no `make`
            // call takes an id, so the ids are the uninterrupted session's.
            "[waiting] #3 tools.read({\"path\":\"c\"})",
            "[waiting] #4 tools.read({\"path\":\"d\"})",
            "0",
            "[ 'HELLO', 'C!', 'D!' ]",
            "dumping at every idle point: off",
        ]
    );
    // What cannot be dumped is refused with its path, and `.dump-every` turns itself off.
    let out = frozen(".dump-every on\nconst w = new WeakMap(); tools.x(w); 0\n");
    assert!(out.contains("[not dumped] cannot dump:\n"), "{out}");
    assert!(out.contains("a weak collection"), "{out}");
    assert!(out.contains("dumping at every idle point: off\n"), "{out}");
}

/// Inputs suspended in their own top-level `await` (async bodies, D2617) dump and
/// restore; an expression input still prints its value when it settles, a statement does
/// not, and new inputs are numbered after the restored ones. The waiting inputs are the
/// dump's completion value's properties (D2895), the calls' origins their states (D2894).
#[test]
fn dump_waiting_await_inputs() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let dump = format!("{dir}/repl-await-{}.js", std::process::id());
    let out = repl_with(
        &["--frozen"],
        &format!(
            "const r = await tools.read({{path: 'a'}})\nawait tools.read({{path: 'b'}})\n\
         .dump {dump}\n.clear\n.restore {dump}\nawait tools.read({{path: 'c'}})\n.pending\n\
         .resolve 3 \"bee\"\n.resolve 2 \"hello\"\nr\n.dump-every on\n\
         const x = await tools.read({{path: 'd'}})\n.resolve 4 \"c!\"\n.resolve 5 \"dee\"\nx\n"
        ),
    );
    let text = std::fs::read_to_string(&dump).unwrap();
    std::fs::remove_file(&dump).unwrap();
    assert!(text.starts_with("\"djs-dump\";"), "{text}");
    let lines: Vec<String> = out
        .lines()
        .filter(|l| !(l.starts_with("[dumped ") || l.starts_with("wrote ")))
        // Positions are in the dump, whose layout is the exporter's.
        .map(|l| {
            l.split(&format!(", at {dump}:"))
                .next()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(
        lines,
        [
            "[waiting] #2 tools.read({\"path\":\"a\"})",
            "[waiting] #3 tools.read({\"path\":\"b\"})",
            "a fresh VM",
            &format!("loaded {dump} into a fresh VM"),
            "waiting: #2 tools.read({\"path\":\"a\"})",
            "waiting: #3 tools.read({\"path\":\"b\"})",
            "[waiting] #4 tools.read({\"path\":\"c\"})",
            "#2 tools.read({\"path\":\"a\"})  input 1",
            "#3 tools.read({\"path\":\"b\"})  input 2",
            "#4 tools.read({\"path\":\"c\"})  input 3, at repl:1:10",
            "[input 2] 'bee'",
            "'hello'",
            "dumping at every idle point: on",
            "[waiting] #5 tools.read({\"path\":\"d\"})",
            "[input 3] 'c!'",
            "'dee'",
        ]
    );
    assert!(out.matches("[dumped ").count() >= 2, "{out}");
    // Without --frozen the VM's builtins are not frozen: dumps need them (D2836).
    let out = repl(".dump -\n.dump-every on\n.dump-every\n");
    assert_eq!(
        out,
        "dumps need frozen builtins (D2836): start the REPL with --frozen\n".repeat(2)
            + "dumping at every idle point: off\n"
    );
}
