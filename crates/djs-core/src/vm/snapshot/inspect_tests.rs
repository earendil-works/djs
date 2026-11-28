//! The inspector (D3760): its text pinned for the guide's examples, its limits, and every
//! idle point of the snapshot test programs inspected as text and JSON.

extern crate std;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use super::tests::{base, info, vm_opts};
use super::*;
use crate::compiler::CompileOptions;
use crate::interp::Status;
use crate::value::Value;

const LOG: u32 = 1;
const DEBUGGER: u32 = 2;
const FETCH: u32 = 3;

/// The snapshot `src`, a body, is at its `nth` pause or `fetch` call idle point (from 1), as
/// the docs site writes them: `console`, `debugger` and `fetch` (async, answered with
/// its argument) the host's.
fn snapshot_at(src: &str, nth: usize, sources: bool) -> Vec<u8> {
    let mut vm = Vm::from_base(base(), vm_opts()).unwrap();
    let console = vm.new_object().unwrap();
    let log = vm.new_host_function(LOG, "log").unwrap();
    vm.define(console, "log", log).unwrap();
    vm.define_global("console", Value::object(console)).unwrap();
    let pause = vm.new_async_host_function(DEBUGGER, "debugger").unwrap();
    vm.define_global("debugger", pause).unwrap();
    let fetch = vm.new_async_host_function(FETCH, "fetch").unwrap();
    vm.define_global("fetch", fetch).unwrap();
    let opts = CompileOptions {
        debugger_awaits: true,
        ..CompileOptions::async_body("example.js", &[])
    };
    let main = vm.compile(src.as_bytes(), &opts).unwrap();
    vm.call(main, Value::UNDEFINED, &[]).unwrap();
    let mut idle = 0;
    loop {
        match vm.run(100_000_000) {
            Status::HostCall { call_id, .. } => {
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap()
            }
            Status::Idle => {
                idle += 1;
                if idle == nth {
                    let host = HostNames {
                        functions: vec![
                            (LOG, "console.log".to_string()),
                            (DEBUGGER, "debugger".to_string()),
                            (FETCH, "fetch".to_string()),
                        ],
                        values: vec![("console".to_string(), Value::object(console))],
                        ..Default::default()
                    };
                    let opts = SnapshotOptions {
                        sources,
                        host: Some(host),
                    };
                    return vm.snapshot_with(info(), &opts).unwrap();
                }
                for id in vm.pending_host_calls() {
                    let v = vm
                        .host_call(id)
                        .unwrap()
                        .args
                        .first()
                        .copied()
                        .unwrap_or(Value::UNDEFINED);
                    vm.resolve_host_call(id, Ok(v)).unwrap();
                }
            }
            s => panic!("no idle point {nth}: {s:?}"),
        }
    }
}

/// The text up to its sizes (which change with any change to what djs stores), the
/// hashes masked.
fn pinned(b: &[u8]) -> String {
    let t = describe(b).unwrap();
    let end = t
        .find("named, not stored")
        .and_then(|k| t[k..].find("\n\n").map(|e| k + e + 1))
        .unwrap_or(t.len());
    let mut out = String::new();
    for l in t[..end].lines() {
        let words: Vec<&str> = l
            .split(' ')
            .map(|w| {
                if w.len() == 16 && w.chars().all(|c| c.is_ascii_hexdigit()) {
                    "<hash>"
                } else {
                    w
                }
            })
            .collect();
        out.push_str(&words.join(" "));
        out.push('\n');
    }
    out.replacen(
        &format!("({}, ", super::inspect::view_bytes(b.len())),
        "(N, ",
        1,
    )
}

const FETCH_PAGE: &str = "async function fetchPage(n) {
  const page = { n, items: [n * 10, n * 10 + 1] };
  debugger;            // a tool call would wait here
  return page;
}
const pages = [];
for (let n = 1; n <= 3; n++) {
  pages.push(await fetchPage(n));
  console.log(\"got page\", n, \"of 3\");
}
const items = pages.flatMap((p) => p.items);
console.log(items.join(\" \"));
return items;";

#[test]
fn the_guides_example_as_a_tree() {
    let b = snapshot_at(FETCH_PAGE, 1, true);
    let want = "example.js, paused at debugger; (N, 21 records)

a pause at debugger; (open call 1): the run waits until it is continued
  · open call 1 is record #6 (HostCall), its promise #8
  fetchPage(n = 1) is stopped at `debugger;` (line 3)
       3 |   debugger;            // a tool call would wait here
         |   ^
    page = { n: 1, items: [10, 11] }
    · activation #10 (Object.Generator, awaiting) continues at pc 45; its frame #11
    · its frame holds callee, this, 1 argument, 2 locals, nothing on the operand stack
    · its promise #12
  the body is waiting for fetchPage, at `await fetchPage(n)` (line 8), halfway through
    pages.push(…)
       8 |   pages.push(await fetchPage(n));
         |              ^
    fetchPage = function fetchPage, pages = [], n = 1, items = undefined
    · activation #17 (Object.Generator, awaiting) continues at pc 32; its frame #18
    · its frame holds callee, this, 0 arguments, 5 locals, operand stack
        Array.prototype.push, #20
    · its promise #3
  the run's result is a promise that is still pending
    · promise #3 (Object.Promise), pending

on the global object: console, debugger(), fetch()
named, not stored (a loading djs must have them):
  the code of fetchPage and the body: the same bytecode, since they are stopped halfway
    (checked by hash)
  builtins every djs has: Object.prototype, Promise.prototype, %AsyncFunctionPrototype%,
    Array.prototype, Array.prototype.push, Function.prototype
  from the host: console, debugger(), fetch()
  · fetchPage is example.js 0..151, bytecode hash <hash>
  · the body is example.js 0..363, bytecode hash <hash>
";
    assert_eq!(pinned(&b), want);
    // The second pause: one more page, the loop at n = 2.
    let t = describe(&snapshot_at(FETCH_PAGE, 2, true)).unwrap();
    assert!(
        t.contains("fetchPage(n = 2) is stopped at `debugger;` (line 3)"),
        "{t}"
    );
    assert!(
        t.contains("pages = [{ n: 1, items: [10, 11] }], n = 2"),
        "{t}"
    );
}

const RICH: &str = "const cache = new Map([[\"a\", 1]]);
let hits = 0;
function remember(key) {
  return async (v) => { hits++; cache.set(key, v); return v; };
}
async function load(name) {
  const v = await fetch(name.length);
  return remember(name)(v);
}
const both = await Promise.all([load(\"alpha\"), load(\"beta\")]);
return [both, hits, [...cache.keys()]];";

#[test]
fn closures_boxes_and_promise_all() {
    let b = snapshot_at(RICH, 1, true);
    let t = pinned(&b);
    let want = "example.js, waiting for the host to answer 2 calls (N, 44 records)

fetch(5) (open call 1): the run waits for the host's answer
  · open call 1 is record #6 (HostCall), its promise #9
  load(name = \"alpha\") is waiting for fetch(5), at `await fetch(name.length)` (line 7)
       7 |   const v = await fetch(name.length);
         |             ^
    v = undefined
    from outside: remember = function remember
    · activation #14 (Object.Generator, awaiting) continues at pc 23; its frame #16
    · its frame holds callee, this, 1 argument, 2 locals, nothing on the operand stack
    · boxes: remember in #23
    · its promise #17
  its promise is the 1st of 2 given to Promise.all, which has 0 of 2 results so far
    · element function #25 (BuiltinClosure); results so far in #36; the combined promise
        #31
  the body is waiting for Promise.all, at `await Promise.all([…])` (line 10)
      10 | const both = await Promise.all([load(\"alpha\"), load(\"beta\")]);
         |              ^
    cache = Map(1) { \"a\" => 1 }, hits = 0, remember = function remember,
      load = function load, both = undefined
    · activation #40 (Object.Generator, awaiting) continues at pc 87; its frame #42
    · its frame holds callee, this, 0 arguments, 6 locals, nothing on the operand stack
    · boxes: cache in #35, hits in #34, remember in #23
    · its promise #3
  the run's result is a promise that is still pending
    · promise #3 (Object.Promise), pending

fetch(4) (open call 2): the run waits for the host's answer
  · open call 2 is record #7 (HostCall), its promise #11
  load(name = \"beta\") is waiting for fetch(4), at `await fetch(name.length)` (line 7)
       7 |   const v = await fetch(name.length);
         |             ^
    v = undefined
    from outside: remember = function remember
    · activation #15 (Object.Generator, awaiting) continues at pc 23; its frame #18
    · its frame holds callee, this, 1 argument, 2 locals, nothing on the operand stack
    · boxes: remember in #23
    · its promise #19
  its promise is the 2nd of 2 given to Promise.all, which has 0 of 2 results so far
    · element function #28 (BuiltinClosure); results so far in #36; the combined promise
        #31
  from there on, as above
    · promise #31, shown above

on the global object: console, debugger(), fetch()
named, not stored (a loading djs must have them):
  the code of load and the body: the same bytecode, since they are stopped halfway
    (checked by hash)
  for remember, held only as a value: the same parameters and captured variables (the
    code may differ)
  builtins every djs has: Object.prototype, Promise.prototype, %AsyncFunctionPrototype%,
    Function.prototype, Array.prototype, Map.prototype
  builtin functions by kind: PromiseAllResolveElement, PromiseRejectFunction
  from the host: console, debugger(), fetch()
  · load is example.js 140..235, bytecode hash <hash>
  · remember is example.js 49..139, interface hash <hash>
  · the body is example.js 0..338, bytecode hash <hash>
";
    assert_eq!(t, want);
    // The JSON has the same tree, and every part.
    let j = inspect(&b).unwrap().json();
    valid_json(&j);
    for s in [
        "\"kind\":\"element\",\"record\":25,\"combinator\":\"Promise.all\",\"index\":0",
        "\"captures\":[{\"name\":\"remember\",\"value\":{\"t\":\"record\",\"v\":23}}]",
        "\"args\":[{\"name\":\"name\",\"value\":{\"t\":\"string\",\"v\":\"alpha\"}}]",
        "\"type\":\"Object.Map\"",
        "\"name\":\"remember\",\"label\":\"remember\"",
        "\"params\":[\"key\"]",
        "\"kind\":\"end\",\"promise\":3,\"state\":\"pending\",\"result\":true",
    ] {
        assert!(j.contains(s), "{s} not in {j}");
    }
}

/// A host's names are stored once, though the global object's keys name them too.
#[test]
fn host_names_once() {
    let b = snapshot_at(FETCH_PAGE, 1, true);
    let j = inspect(&b).unwrap().json();
    let strings = &j[j.find("\"strings\":[").unwrap()..];
    let strings = &strings[..strings.find(']').unwrap()];
    for n in ["\"console\"", "\"debugger\"", "\"fetch\""] {
        assert_eq!(strings.matches(n).count(), 1, "{n} in {strings}");
    }
}

/// Large data is cut at the text's limits; the JSON has it all.
#[test]
fn large_runs_are_cut() {
    let src = "const big = Array.from({ length: 1000 }, (_, i) => ({ i, s: 'x'.repeat(i % 100) }));
const deep = { a: { b: { c: { d: { e: 1 } } } } };
const long = 'y'.repeat(500);
async function f(k) { await fetch(k); }
await Promise.all(Array.from({ length: 40 }, (_, k) => f(k)));";
    let b = snapshot_at(src, 1, true);
    let t = describe(&b).unwrap();
    assert!(t.contains(" more]"), "{t}");
    assert!(t.contains("(500 chars)"), "{t}");
    assert!(t.contains("waiting for the host to answer 40 calls"), "{t}");
    assert!(t.contains("(the first 16 open calls in full)"), "{t}");
    assert!(t.lines().count() < 400, "{} lines", t.lines().count());
    assert!(t.lines().all(|l| l.chars().count() < 200), "{t}");
    let j = inspect(&b).unwrap().json();
    valid_json(&j);
    assert!(j.contains(&"y".repeat(500)));
}

/// Without its source a snapshot still shows its frames, by their slots.
#[test]
fn without_sources_frames_show_their_slots() {
    let b = snapshot_at(FETCH_PAGE, 1, false);
    let t = describe(&b).unwrap();
    assert!(t.contains("a pause at debugger; (open call 1)"), "{t}");
    assert!(t.contains("is stopped at pc 45"), "{t}");
    assert!(
        t.contains("no source to name it by: this and its slots "),
        "{t}"
    );
    valid_json(&inspect(&b).unwrap().json());
}

/// A generator stopped at a `yield` and an async function awaiting a promise nothing
/// settles: activations no open call leads to.
#[test]
fn activations_no_call_leads_to() {
    let src = "function* g() { let i = 0; while (true) yield i++; }
const it = g(); it.next(); it.next();
let resolve;
const never = (async () => { const w = 'waits'; await new Promise((r) => { resolve = r; }); })();
const p = Promise.resolve(1).then(async (x) => { await fetch(x); return x + 1; });
const q = p.then((y) => y * 2);
await q;";
    let b = snapshot_at(src, 1, true);
    let t = describe(&b).unwrap();
    for s in [
        "also suspended, not waiting for the host:",
        "g() is stopped at `yield i++` (line 1)",
        "i = 2",
        "w = \"waits\"",
        "nothing waits for it yet (a promise that is still pending)",
        "then the callback (y) => y * 2 runs (a .then reaction)",
        "then another promise takes on its result",
        "the body is waiting for the .then callback, at `await q` (line 7)",
        "resolve = ‹a promise's resolve function›",
    ] {
        assert!(t.contains(s), "{s} not in {t}");
    }
    valid_json(&inspect(&b).unwrap().json());
}

/// Every idle point of the snapshot test programs inspects, as text and as valid JSON,
/// and every open call leads somewhere.
#[test]
fn every_program_idle_point_inspects() {
    let mut points = 0;
    for (name, src) in super::tests::programs() {
        super::tests::run_program(&src, &mut |vm, k| {
            let Ok(b) = vm.snapshot(info()) else { return };
            points += 1;
            let i = inspect(&b).unwrap_or_else(|e| panic!("{name}@{k}: {:?}", e.0));
            let t = i.text();
            assert!(t.starts_with("src.js, "), "{name}@{k}: {t}");
            assert!(
                !t.contains("open call 0") && !t.contains("?("),
                "{name}@{k}: {t}"
            );
            valid_json(&i.json());
            if std::env::var_os("DJS_INSPECT_OUT").is_some() {
                std::println!("== {name}@{k}\n{t}");
            }
        });
    }
    assert!(points > 50, "{points} idle points");
}

/// A JSON value (a check, not a parser): panics where `s` is not one.
fn valid_json(s: &str) {
    let b = s.as_bytes();
    let mut at = 0;
    value(b, &mut at);
    assert_eq!(
        at,
        b.len(),
        "trailing text at {at}: {}",
        &s[at.saturating_sub(40)..]
    );

    fn ws(b: &[u8], at: &mut usize) {
        while *at < b.len() && b[*at].is_ascii_whitespace() {
            *at += 1;
        }
    }
    fn lit(b: &[u8], at: &mut usize, w: &str) {
        assert!(b[*at..].starts_with(w.as_bytes()), "bad JSON at {at}");
        *at += w.len();
    }
    fn string(b: &[u8], at: &mut usize) {
        lit(b, at, "\"");
        while b[*at] != b'"' {
            assert!(b[*at] >= 0x20, "control character at {at}");
            *at += if b[*at] == b'\\' { 2 } else { 1 };
        }
        *at += 1;
    }
    fn value(b: &[u8], at: &mut usize) {
        ws(b, at);
        match b[*at] {
            b'{' | b'[' => {
                let close = if b[*at] == b'{' { b'}' } else { b']' };
                *at += 1;
                ws(b, at);
                if b[*at] == close {
                    *at += 1;
                    return;
                }
                loop {
                    if close == b'}' {
                        ws(b, at);
                        string(b, at);
                        ws(b, at);
                        lit(b, at, ":");
                    }
                    value(b, at);
                    ws(b, at);
                    match b[*at] {
                        b',' => *at += 1,
                        c if c == close => {
                            *at += 1;
                            return;
                        }
                        _ => panic!("bad JSON at {at}"),
                    }
                }
            }
            b'"' => string(b, at),
            b't' => lit(b, at, "true"),
            b'f' => lit(b, at, "false"),
            b'n' => lit(b, at, "null"),
            _ => {
                let from = *at;
                while *at < b.len()
                    && matches!(b[*at], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
                {
                    *at += 1;
                }
                assert!(*at > from, "bad JSON at {from}");
            }
        }
    }
}
