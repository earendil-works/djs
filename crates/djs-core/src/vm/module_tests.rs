//! The module API (vm/module.rs): loading, linking, evaluation, namespaces, exports.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::{format, vec};

use super::*;

/// A loader over `(key, source)` pairs; specifiers are keys. Counts loads.
struct Map {
    files: Vec<(&'static str, &'static str)>,
    loads: Vec<String>,
}

impl ModuleLoader for Map {
    fn resolve(&mut self, specifier: &str, _referrer: Option<&str>) -> Result<String, String> {
        if specifier.starts_with("bad:") {
            return Err(format!("cannot resolve {specifier}"));
        }
        Ok(specifier.to_string())
    }

    fn load(&mut self, key: &str) -> Result<ModuleSource, String> {
        self.loads.push(key.to_string());
        let text = self
            .files
            .iter()
            .find(|f| f.0 == key)
            .ok_or_else(|| format!("no module {key}"))?
            .1;
        Ok(ModuleSource {
            text: text.to_string(),
            typescript: key.ends_with(".ts"),
        })
    }
}

fn map(files: &[(&'static str, &'static str)]) -> Map {
    Map {
        files: files.to_vec(),
        loads: Vec::new(),
    }
}

/// Runs to the end (images in between when `suspend`): the settled promise's outcome.
fn settle(vm: &mut Vm, suspend: bool) -> String {
    loop {
        match vm.run(if suspend { 7 } else { 1_000_000 }) {
            Status::Done(p) => {
                return match vm.promise_state(p) {
                    Some(Some(Ok(v))) => format!("ok {}", vm.display(v)),
                    Some(Some(Err(e))) => format!("err {}", vm.display(e)),
                    other => format!("{other:?}"),
                };
            }
            Status::OutOfFuel if suspend => {
                let opts = vm.options();
                *vm = Vm::from_bytes(vm.image().to_vec(), opts).unwrap();
            }
            Status::OutOfFuel => {}
            s => return format!("{s:?}"),
        }
    }
}

fn eval(vm: &mut Vm, key: &str) -> String {
    assert!(vm.evaluate_module(key).unwrap());
    settle(vm, false)
}

fn export(vm: &mut Vm, key: &str, name: &str) -> String {
    match vm.module_export(key, name).unwrap() {
        Some(v) => vm.display(v),
        None => String::from("-"),
    }
}

/// Calls exported function `name` of `key` with no arguments: its result.
fn call_export(vm: &mut Vm, key: &str, name: &str) -> String {
    let f = vm.module_export(key, name).unwrap().unwrap();
    vm.call(f, Value::UNDEFINED, &[]).unwrap();
    match vm.run(1_000_000) {
        Status::Done(v) => vm.display(v),
        Status::Threw(e) => format!("throw {}", vm.display(e)),
        s => format!("{s:?}"),
    }
}

#[test]
fn live_bindings_and_namespaces() {
    let mut vm = Vm::new(VmOptions::default()).unwrap();
    let mut l = map(&[
        (
            "main",
            "import { count, inc } from 'counter';
             import * as ns from 'counter';
             import def from 'counter';
             inc(); inc();
             export const seen = [count, ns.count, def, typeof ns, Object.keys(ns).join(),
                                  Object.prototype.toString.call(ns), this === undefined];
             export { count };",
        ),
        (
            "counter",
            "export let count = 0;
             export function inc() { count++; }
             export default 'd';",
        ),
    ]);
    let key = vm.load_module("main", &mut l).unwrap();
    assert_eq!(key, "main");
    assert_eq!(l.loads, ["main", "counter"]);
    assert_eq!(eval(&mut vm, "main"), "ok undefined");
    assert_eq!(
        export(&mut vm, "main", "seen"),
        "[2,2,\"d\",\"object\",\"count,default,inc\",\"[object Module]\",true]"
    );
    // Live: the re-export reads the binding.
    call_export(&mut vm, "counter", "inc");
    assert_eq!(export(&mut vm, "main", "count"), "3");
    // A second load of the same key loads nothing.
    let mut l2 = map(&[]);
    vm.load_module("main", &mut l2).unwrap();
    assert!(l2.loads.is_empty());
    // A second evaluation runs nothing.
    assert_eq!(eval(&mut vm, "main"), "ok undefined");
    assert_eq!(export(&mut vm, "counter", "count"), "3");
    let ns = vm.module_namespace("counter").unwrap().unwrap();
    vm.define_global("ns", ns).unwrap();
    let f = vm
        .compile(
            b"const r = [];
              try { ns.count = 1 } catch (e) { r.push(e.constructor.name) }
              r.push(Object.isExtensible(ns), Reflect.set(ns, 'count', 1), 'count' in ns,
                     Reflect.deleteProperty(ns, 'count'), delete ns.nope, Reflect.getPrototypeOf(ns),
                     JSON.stringify(Object.getOwnPropertyDescriptor(ns, 'count')),
                     Reflect.defineProperty(ns, 'count', { value: 3 }),
                     Reflect.defineProperty(ns, 'count', { value: 4 }),
                     ns[Symbol.toStringTag]);
              r",
            &crate::compiler::CompileOptions::script("t.js"),
        )
        .unwrap();
    let g = Value::object(vm.global());
    vm.call(f, g, &[]).unwrap();
    let v = match vm.run(1_000_000) {
        Status::Done(v) => v,
        Status::Threw(e) => panic!("{}", vm.display(e)),
        s => panic!("{s:?}"),
    };
    assert_eq!(
        vm.display(v),
        "[\"TypeError\",false,false,true,false,true,null,\
         \"{\\\"value\\\":3,\\\"writable\\\":true,\\\"enumerable\\\":true,\\\"configurable\\\":false}\",\
         true,false,\"Module\"]"
    );
}

#[test]
fn cycles_and_hoisted_functions() {
    let mut vm = Vm::new(VmOptions::default()).unwrap();
    let mut l = map(&[
        (
            "a",
            "import { b, fromB } from 'b';
             export function a() { return 'a' + b(); }
             export const order = [];
             order.push('a', fromB);",
        ),
        (
            "b",
            // `a` runs before a's body: a hoisted function of a module not run yet.
            "import { a, order } from 'a';
             export function b() { return 'b'; }
             export const fromB = a();
             let early;
             try { order } catch (e) { early = e.name }
             export { early };",
        ),
    ]);
    vm.load_module("a", &mut l).unwrap();
    assert_eq!(eval(&mut vm, "a"), "ok undefined");
    assert_eq!(export(&mut vm, "a", "order"), "[\"a\",\"ab\"]");
    assert_eq!(export(&mut vm, "b", "early"), "ReferenceError");
}

#[test]
fn link_and_load_errors() {
    let mut vm = Vm::new(VmOptions::default()).unwrap();
    let cases: &[(&[(&'static str, &'static str)], &str)] = &[
        (
            &[("m", "import { x } from 'n'"), ("n", "export const y = 1")],
            "SyntaxError: Could not find export 'x' in module 'n'",
        ),
        (
            &[
                ("m", "import { x } from 'n'"),
                ("n", "export * from 'o'; export * from 'p'"),
                ("o", "export const x = 1"),
                ("p", "export const x = 2"),
            ],
            "SyntaxError: export 'x' in module 'n' is ambiguous",
        ),
        (
            &[("m", "export { x } from 'n'"), ("n", "export {}")],
            "SyntaxError: Could not find export 'x' in module 'n'",
        ),
        (
            &[("m", "import 'n'"), ("n", "let let = 1")],
            "SyntaxError: ",
        ),
        (&[("m", "import 'n'")], "Error: no module n"),
        (&[("m", "import 'bad:x'")], "Error: cannot resolve bad:x"),
    ];
    for &(files, msg) in cases {
        let mut l = map(files);
        let e = vm.load_module("m", &mut l).unwrap_err();
        let text = vm.display(e);
        assert!(text.starts_with(msg), "{files:?}: {text}");
        // Nothing stays loaded.
        assert!(!vm.has_module("m"));
    }
    // A star export of the same binding twice is not ambiguous; `default` never comes
    // through a star; a namespace leaves ambiguous names out.
    let mut l = map(&[
        (
            "m",
            "import * as ns from 'n'; export const keys = Object.keys(ns).join()",
        ),
        (
            "n",
            "export * from 'o'; export * from 'p'; export * from 'q'",
        ),
        ("o", "export { x, z } from 'q'; export default 1"),
        ("p", "export * from 'q'; export const w = 1"),
        ("q", "export const x = 1, z = 2; export const w = 3"),
    ]);
    vm.load_module("m", &mut l).unwrap();
    assert_eq!(eval(&mut vm, "m"), "ok undefined");
    assert_eq!(export(&mut vm, "m", "keys"), "x,z");
}

#[test]
fn evaluation_errors_are_kept() {
    let mut vm = Vm::new(VmOptions::default()).unwrap();
    let mut l = map(&[
        ("m", "import 'n'; export const after = 1"),
        ("n", "export const x = 1; throw new TypeError('boom')"),
        ("k", "import { x } from 'n'"),
    ]);
    vm.load_module("m", &mut l).unwrap();
    assert_eq!(eval(&mut vm, "m"), "err TypeError: boom");
    assert_eq!(eval(&mut vm, "m"), "err TypeError: boom");
    assert_eq!(export(&mut vm, "m", "after"), "-");
    // Another importer of the failed module gets its error too.
    vm.load_module("k", &mut l).unwrap();
    assert_eq!(eval(&mut vm, "k"), "err TypeError: boom");
}

#[test]
fn import_meta_and_typescript() {
    let mut vm = Vm::new(VmOptions::default()).unwrap();
    let mut l = map(&[
        (
            "main.ts",
            "import { type T, f } from 'lib.ts';
             import type { U } from 'types.ts';
             import { Only } from 'types.ts';
             let x: T = f();
             export const url = import.meta.url, same = import.meta === (() => import.meta)();
             export { x, type U };",
        ),
        (
            "lib.ts",
            "export type T = number; export function f(): T { return 7 }",
        ),
        ("types.ts", "export interface U {} export type Only = 1;"),
    ]);
    vm.load_module("main.ts", &mut l).unwrap();
    // The type-only imports load nothing.
    assert_eq!(l.loads, ["main.ts", "lib.ts"]);
    assert_eq!(eval(&mut vm, "main.ts"), "ok undefined");
    assert_eq!(export(&mut vm, "main.ts", "url"), "main.ts");
    assert_eq!(export(&mut vm, "main.ts", "same"), "true");
    assert_eq!(export(&mut vm, "main.ts", "x"), "7");
}

#[test]
fn graphs_survive_images_and_suspension() {
    let mut vm = Vm::new(VmOptions::default()).unwrap();
    let mut l = map(&[
        (
            "m",
            "import { n, bump } from 'n';
             let s = 0;
             for (let i = 0; i < 50; i++) s += i;
             export const total = s + n;
             export function next() { bump(); return n; }",
        ),
        ("n", "export let n = 1; export function bump() { n++; }"),
    ]);
    vm.load_module("m", &mut l).unwrap();
    assert!(vm.evaluate_module("m").unwrap());
    // Out of fuel many times, through an image each time.
    assert_eq!(settle(&mut vm, true), "ok undefined");
    vm.collect().unwrap();
    let opts = vm.options();
    let mut vm2 = Vm::from_bytes(vm.image().to_vec(), opts).unwrap();
    assert_eq!(export(&mut vm2, "m", "total"), "1226");
    assert_eq!(call_export(&mut vm2, "m", "next"), "2");
    assert_eq!(export(&mut vm2, "n", "n"), "2");
    assert_eq!(call_export(&mut vm2, "m", "next"), "3");
    // The original VM is unaffected.
    assert_eq!(export(&mut vm, "n", "n"), "1");
    let _ = vec![0];
}
