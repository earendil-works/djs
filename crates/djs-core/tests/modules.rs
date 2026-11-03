//! Native module binding graph tests. These do not claim source-module execution.
use djs_core::value::Value;
use djs_core::vm::modules::{BindingKind, ModuleError, ModuleGraph};
use djs_core::vm::{Status, Vm, VmOptions};

fn vm() -> Vm {
    Vm::new(VmOptions {
        prelude: false,
        ..VmOptions::default()
    })
    .unwrap()
}

fn export(graph: &ModuleGraph, vm: &mut Vm, id: &str, name: &str, kind: BindingKind) {
    graph.declare_binding(vm, id, name, kind).unwrap();
    graph.export(vm, id, name, name).unwrap();
}

#[test]
fn graph_shares_live_cells_through_imports_and_reexports() {
    let mut vm = vm();
    let graph = ModuleGraph::new(&mut vm).unwrap();
    for id in ["counter", "barrel", "consumer"] {
        graph.declare(&mut vm, id).unwrap();
    }
    export(&graph, &mut vm, "counter", "count", BindingKind::Mutable);
    graph
        .import(&mut vm, "barrel", "count", "counter", "count")
        .unwrap();
    graph.export(&mut vm, "barrel", "default", "count").unwrap();
    graph
        .import(&mut vm, "consumer", "current", "barrel", "default")
        .unwrap();
    assert_eq!(
        graph.read(&mut vm, "consumer", "current"),
        Err(ModuleError::Uninitialized)
    );
    graph
        .initialize(&mut vm, "counter", "count", Value::int(1))
        .unwrap();
    assert_eq!(
        graph.read(&mut vm, "consumer", "current").unwrap(),
        Value::int(1)
    );
    graph
        .set(&mut vm, "counter", "count", Value::int(2))
        .unwrap();
    assert_eq!(
        graph.read_export(&mut vm, "barrel", "default").unwrap(),
        Value::int(2)
    );
    assert_eq!(
        graph.read(&mut vm, "consumer", "current").unwrap(),
        Value::int(2)
    );
    for id in ["barrel", "consumer"] {
        let local = if id == "barrel" { "count" } else { "current" };
        assert_eq!(
            graph.set(&mut vm, id, local, Value::int(3)),
            Err(ModuleError::ReadOnly)
        );
        assert_eq!(
            graph.initialize(&mut vm, id, local, Value::int(3)),
            Err(ModuleError::ReadOnly)
        );
    }
}

#[test]
fn cyclic_bindings_link_before_initialization() {
    let mut vm = vm();
    let graph = ModuleGraph::new(&mut vm).unwrap();
    graph.declare(&mut vm, "a").unwrap();
    graph.declare(&mut vm, "b").unwrap();
    export(&graph, &mut vm, "a", "a", BindingKind::Mutable);
    export(&graph, &mut vm, "b", "b", BindingKind::Constant);
    graph.import(&mut vm, "a", "b", "b", "b").unwrap();
    graph.import(&mut vm, "b", "a", "a", "a").unwrap();
    assert_eq!(
        graph.read(&mut vm, "a", "b"),
        Err(ModuleError::Uninitialized)
    );
    assert_eq!(
        graph.read(&mut vm, "b", "a"),
        Err(ModuleError::Uninitialized)
    );
    graph.initialize(&mut vm, "a", "a", Value::int(10)).unwrap();
    graph.initialize(&mut vm, "b", "b", Value::int(20)).unwrap();
    assert_eq!(graph.read(&mut vm, "b", "a").unwrap(), Value::int(10));
    assert_eq!(graph.read(&mut vm, "a", "b").unwrap(), Value::int(20));
    graph.set(&mut vm, "a", "a", Value::int(30)).unwrap();
    assert_eq!(graph.read(&mut vm, "b", "a").unwrap(), Value::int(30));
}

#[test]
fn errors_preserve_existing_bindings_and_identity() {
    let mut vm = vm();
    let graph = ModuleGraph::new(&mut vm).unwrap();
    assert!(!graph.contains(&mut vm, "m").unwrap());
    graph.declare(&mut vm, "m").unwrap();
    assert!(graph.contains(&mut vm, "m").unwrap());
    assert_eq!(
        graph.declare(&mut vm, "m"),
        Err(ModuleError::DuplicateModule)
    );
    export(&graph, &mut vm, "m", "fixed", BindingKind::Constant);
    assert_eq!(
        graph.set(&mut vm, "m", "fixed", Value::int(1)),
        Err(ModuleError::Uninitialized)
    );
    graph
        .initialize(&mut vm, "m", "fixed", Value::UNDEFINED)
        .unwrap();
    assert_eq!(
        graph.initialize(&mut vm, "m", "fixed", Value::int(1)),
        Err(ModuleError::AlreadyInitialized)
    );
    assert_eq!(
        graph.set(&mut vm, "m", "fixed", Value::int(1)),
        Err(ModuleError::ReadOnly)
    );
    assert_eq!(
        graph.declare_binding(&mut vm, "m", "fixed", BindingKind::Mutable),
        Err(ModuleError::DuplicateBinding)
    );
    assert_eq!(
        graph.export(&mut vm, "m", "fixed", "fixed"),
        Err(ModuleError::DuplicateBinding)
    );
    assert_eq!(
        graph.read(&mut vm, "absent", "x"),
        Err(ModuleError::MissingModule)
    );
    assert_eq!(
        graph.read_export(&mut vm, "m", "absent"),
        Err(ModuleError::MissingBinding)
    );
    assert_eq!(
        graph.import(&mut vm, "m", "new", "m", "absent"),
        Err(ModuleError::MissingBinding)
    );
    assert_eq!(
        graph.read(&mut vm, "m", "new"),
        Err(ModuleError::MissingBinding)
    );
    assert_eq!(
        graph.read_export(&mut vm, "m", "fixed").unwrap(),
        Value::UNDEFINED
    );
}

#[test]
fn identities_and_binding_names_have_no_prototype_or_reserved_names() {
    let mut vm = vm();
    let graph = ModuleGraph::new(&mut vm).unwrap();
    for name in [
        "__proto__",
        "constructor",
        "",
        "0",
        "locals",
        "exports",
        "cell",
        "value",
    ] {
        graph.declare(&mut vm, name).unwrap();
        export(&graph, &mut vm, name, name, BindingKind::Mutable);
        graph
            .initialize(&mut vm, name, name, Value::int(7))
            .unwrap();
        assert_eq!(
            graph.read_export(&mut vm, name, name).unwrap(),
            Value::int(7)
        );
    }
    assert!(!graph.contains(&mut vm, "toString").unwrap());
}

#[test]
fn live_aliases_and_tdz_survive_collections_and_images() {
    let mut vm = vm();
    let graph = ModuleGraph::new(&mut vm).unwrap();
    for id in ["origin", "consumer"] {
        graph.declare(&mut vm, id).unwrap();
    }
    export(&graph, &mut vm, "origin", "value", BindingKind::Mutable);
    export(&graph, &mut vm, "origin", "later", BindingKind::Constant);
    graph
        .import(&mut vm, "consumer", "value", "origin", "value")
        .unwrap();
    graph
        .export(&mut vm, "consumer", "default", "value")
        .unwrap();
    let object = vm.new_object().unwrap();
    graph
        .initialize(&mut vm, "origin", "value", Value::object(object))
        .unwrap();
    let index = graph.handle_index();
    for _ in 0..3 {
        vm.collect().unwrap();
        vm.collect_canonical().unwrap();
        let image = vm.bytes().to_vec();
        vm = Vm::from_bytes(image, vm.options()).unwrap();
        let restored = ModuleGraph::from_handle_index(index);
        let original = restored.read_export(&mut vm, "origin", "value").unwrap();
        assert!(original.as_object().is_some());
        assert_eq!(
            restored
                .read_export(&mut vm, "consumer", "default")
                .unwrap(),
            original
        );
        assert_eq!(
            restored.read_export(&mut vm, "origin", "later"),
            Err(ModuleError::Uninitialized)
        );
    }
    graph
        .set(&mut vm, "origin", "value", Value::int(42))
        .unwrap();
    assert_eq!(
        graph.read(&mut vm, "consumer", "value").unwrap(),
        Value::int(42)
    );
    assert_eq!(
        graph.set(&mut vm, "origin", "value", Value::UNINITIALIZED),
        Err(ModuleError::InvalidValue)
    );
    assert_eq!(
        graph.read(&mut vm, "consumer", "value").unwrap(),
        Value::int(42)
    );
    graph.release(&mut vm);
    vm.collect().unwrap();
    assert_eq!(vm.heap().handle_count(), 0);
}

#[test]
fn synthetic_export_can_hold_a_callable_host_function() {
    let mut vm = vm();
    let graph = ModuleGraph::new(&mut vm).unwrap();
    graph.declare(&mut vm, "host:tools").unwrap();
    export(&graph, &mut vm, "host:tools", "run", BindingKind::Constant);
    let function = vm.new_host_function(27, "run").unwrap();
    graph
        .initialize(&mut vm, "host:tools", "run", function)
        .unwrap();
    vm.collect().unwrap();
    let function = graph.read_export(&mut vm, "host:tools", "run").unwrap();
    vm.call(function, Value::UNDEFINED, &[Value::int(4)])
        .unwrap();
    let Status::HostCall { call_id: call, .. } = vm.run(1000) else {
        panic!("exported host function did not suspend")
    };
    let info = vm.host_call(call).unwrap();
    assert_eq!(info.fn_id, 27);
    assert_eq!(info.args, [Value::int(4)]);
    let index = graph.handle_index();
    let image = vm.bytes().to_vec();
    vm = Vm::from_bytes(image, vm.options()).unwrap();
    let graph = ModuleGraph::from_handle_index(index);
    vm.resolve_host_call(call, Ok(Value::int(8))).unwrap();
    assert!(matches!(vm.run(1000), Status::Done(v) if v == Value::int(8)));
    assert!(
        graph
            .read_export(&mut vm, "host:tools", "run")
            .unwrap()
            .as_object()
            .is_some()
    );
}

#[test]
fn synthetic_async_function_keeps_pending_promise_through_snapshot() {
    let mut vm = djs_prelude::new_vm(VmOptions::default()).unwrap();
    let graph = ModuleGraph::new(&mut vm).unwrap();
    graph.declare(&mut vm, "host:async").unwrap();
    export(&graph, &mut vm, "host:async", "run", BindingKind::Constant);
    let function = vm.new_async_host_function(28, "run").unwrap();
    graph
        .initialize(&mut vm, "host:async", "run", function)
        .unwrap();
    vm.call(function, Value::UNDEFINED, &[]).unwrap();
    assert_eq!(vm.run(1000), Status::Idle);
    let calls = vm.new_host_calls();
    assert_eq!(calls.len(), 1);
    let call = calls[0];
    assert!(vm.host_call(call).unwrap().is_async);
    vm.collect().unwrap();
    let image = vm.bytes().to_vec();
    vm = Vm::from_bytes(image, vm.options()).unwrap();
    vm.resolve_host_call(call, Ok(Value::int(9))).unwrap();
    let Status::Done(promise) = vm.run(1000) else {
        panic!("async synthetic function did not settle")
    };
    assert_eq!(vm.promise_state(promise), Some(Some(Ok(Value::int(9)))));
    assert!(
        graph
            .read_export(&mut vm, "host:async", "run")
            .unwrap()
            .as_object()
            .is_some()
    );
}

struct Sources {
    entries: Vec<(&'static str, &'static str)>,
    loads: Vec<String>,
    resolves: Vec<(String, String)>,
}

impl Sources {
    fn new(entries: Vec<(&'static str, &'static str)>) -> Self {
        Self {
            entries,
            loads: Vec::new(),
            resolves: Vec::new(),
        }
    }
}
impl djs_core::vm::modules::ModuleLoader for Sources {
    fn resolve(
        &mut self,
        referrer: &str,
        specifier: &str,
    ) -> Result<String, djs_core::vm::modules::SourceError> {
        self.resolves.push((referrer.into(), specifier.into()));
        Ok(specifier.trim_start_matches("./").into())
    }
    fn load(&mut self, canonical: &str) -> Result<String, djs_core::vm::modules::SourceError> {
        self.loads.push(canonical.into());
        self.entries
            .iter()
            .find(|(name, _)| *name == canonical)
            .map(|(_, source)| String::from(*source))
            .ok_or_else(|| {
                djs_core::vm::modules::SourceError::Loader(format!("missing source {canonical}"))
            })
    }
}

fn evaluate(graph: &ModuleGraph, vm: &mut Vm, id: &str) -> Status {
    graph.begin_evaluation(vm, id).unwrap();
    for _ in 0..10000 {
        match graph.poll_evaluation(vm, 1000).unwrap() {
            Status::OutOfFuel => {}
            other => return other,
        }
    }
    panic!("module graph did not finish")
}

#[test]
fn source_imports_exported_functions_observe_live_bindings() {
    let mut vm = vm();
    let graph = ModuleGraph::new(&mut vm).unwrap();
    let mut loader = Sources::new(vec![
        (
            "counter.js",
            "export let count = 0; export function inc() { count++; } export function read() { return count; }",
        ),
        (
            "main.js",
            "import {count, inc, read} from './counter.js'; export let before = count; inc(); export const after = count; export const viaClosure = read(); export {count as live}; export const receiver = this;",
        ),
    ]);
    graph.load_source(&mut vm, "main.js", &mut loader).unwrap();
    assert_eq!(
        graph.read_export(&mut vm, "main.js", "after"),
        Err(ModuleError::Uninitialized)
    );
    assert_eq!(
        evaluate(&graph, &mut vm, "main.js"),
        Status::Done(Value::UNDEFINED)
    );
    assert_eq!(
        graph.read_export(&mut vm, "main.js", "before").unwrap(),
        Value::int(0)
    );
    assert_eq!(
        graph.read_export(&mut vm, "main.js", "after").unwrap(),
        Value::int(1)
    );
    assert_eq!(
        graph.read_export(&mut vm, "main.js", "viaClosure").unwrap(),
        Value::int(1)
    );
    assert_eq!(
        graph.read_export(&mut vm, "main.js", "receiver").unwrap(),
        Value::UNDEFINED
    );
    let increment = graph.read_export(&mut vm, "counter.js", "inc").unwrap();
    vm.call(increment, Value::UNDEFINED, &[]).unwrap();
    assert!(matches!(vm.run(10000), Status::Done(_)));
    assert_eq!(
        graph.read_export(&mut vm, "main.js", "live").unwrap(),
        Value::int(2)
    );
    assert!(
        loader
            .resolves
            .contains(&("main.js".into(), "./counter.js".into()))
    );
}

#[test]
fn source_default_reexports_side_effect_imports_and_once_only_evaluation() {
    let mut vm = vm();
    vm.define_global("runs", Value::int(0)).unwrap();
    let graph = ModuleGraph::new(&mut vm).unwrap();
    let mut loader = Sources::new(vec![
        (
            "value.js",
            "runs++; export default function answer() { return 42; } export let mutable = 1;",
        ),
        (
            "barrel.js",
            "export {default as answer, mutable as current} from './value.js';",
        ),
        (
            "main.js",
            "import './value.js'; import answer, {mutable} from './value.js'; import {answer as again} from './barrel.js'; runs += 10; export default answer() + again() + mutable; export const totalRuns = runs;",
        ),
    ]);
    graph.load_source(&mut vm, "main.js", &mut loader).unwrap();
    assert_eq!(
        evaluate(&graph, &mut vm, "main.js"),
        Status::Done(Value::UNDEFINED)
    );
    assert_eq!(
        graph.read_export(&mut vm, "main.js", "default").unwrap(),
        Value::int(85)
    );
    assert_eq!(
        graph.read_export(&mut vm, "main.js", "totalRuns").unwrap(),
        Value::int(11)
    );
    graph.load_source(&mut vm, "main.js", &mut loader).unwrap();
    assert_eq!(
        evaluate(&graph, &mut vm, "main.js"),
        Status::Done(Value::UNDEFINED)
    );
    assert_eq!(
        loader.loads.iter().filter(|id| *id == "value.js").count(),
        1
    );
    let probe = vm
        .compile(
            b"runs",
            &djs_core::compiler::CompileOptions::script("cache-probe.js"),
        )
        .unwrap();
    vm.call(probe, Value::UNDEFINED, &[]).unwrap();
    assert_eq!(vm.run(1000), Status::Done(Value::int(11)));
    graph
        .set(&mut vm, "value.js", "mutable", Value::int(8))
        .unwrap();
    assert_eq!(
        graph.read_export(&mut vm, "barrel.js", "current").unwrap(),
        Value::int(8)
    );
}

#[test]
fn source_link_errors_happen_before_any_execution_and_are_cached() {
    use djs_core::vm::modules::SourceError;
    for (source, syntax) in [
        ("export const x = 1; export {x};", true),
        ("export {missing};", true),
        ("import {missing} from './dep.js';", false),
        ("import {x} from './dep.js'; const x = 3;", true),
        ("{ import {x} from './dep.js'; }", true),
        ("return 1;", true),
        ("await 1;", true),
    ] {
        let mut vm = vm();
        vm.define_global("runs", Value::int(0)).unwrap();
        let graph = ModuleGraph::new(&mut vm).unwrap();
        let mut loader = Sources::new(vec![
            ("main.js", source),
            ("dep.js", "runs++; export const x = 1;"),
        ]);
        let error = graph
            .load_source(&mut vm, "main.js", &mut loader)
            .unwrap_err();
        if syntax {
            assert!(
                matches!(error, SourceError::Compile(_)),
                "{source}: {error}"
            );
        } else {
            assert!(
                matches!(error, SourceError::Binding(ModuleError::MissingBinding)),
                "{error}"
            );
        }
        assert!(matches!(
            graph.load_source(&mut vm, "main.js", &mut loader),
            Err(SourceError::Cached(_))
        ));
        let value = vm
            .compile(
                b"runs",
                &djs_core::compiler::CompileOptions::script("probe.js"),
            )
            .unwrap();
        vm.call(value, Value::UNDEFINED, &[]).unwrap();
        assert_eq!(vm.run(1000), Status::Done(Value::int(0)));
    }
}

#[test]
fn source_import_assignment_throws_and_failure_is_cached_by_identity() {
    let mut vm = djs_prelude::new_vm(VmOptions::default()).unwrap();
    let graph = ModuleGraph::new(&mut vm).unwrap();
    let mut loader = Sources::new(vec![
        ("dep.js", "export let x = 1;"),
        ("main.js", "import {x} from './dep.js'; x = 9;"),
    ]);
    graph.load_source(&mut vm, "main.js", &mut loader).unwrap();
    let Status::Threw(error) = evaluate(&graph, &mut vm, "main.js") else {
        panic!("import assignment must throw")
    };
    assert!(vm.display(error).contains("TypeError"));
    let root = vm.heap_mut().new_handle(error).unwrap();
    vm.collect().unwrap();
    let expected = vm.heap().get_handle(root);
    assert_eq!(
        evaluate(&graph, &mut vm, "main.js"),
        Status::Threw(expected)
    );
    assert_eq!(
        graph.read_export(&mut vm, "dep.js", "x").unwrap(),
        Value::int(1)
    );
}

#[test]
fn source_execution_and_live_closures_survive_fuel_gc_and_snapshot() {
    let options = VmOptions {
        exec: djs_core::vm::ExecOptions {
            gc_stress: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut vm = djs_prelude::new_vm(options).unwrap();
    let graph = ModuleGraph::new(&mut vm).unwrap();
    let mut loader = Sources::new(vec![
        (
            "dep.js",
            "export let count = 1; export function inc() { count += 1; }",
        ),
        (
            "main.js",
            "import {count, inc} from './dep.js'; for (let i=0; i<8; i++) inc(); export function read() { return count; } export {count};",
        ),
    ]);
    graph.load_source(&mut vm, "main.js", &mut loader).unwrap();
    graph.begin_evaluation(&mut vm, "main.js").unwrap();
    for _ in 0..1000 {
        let result = graph.poll_evaluation(&mut vm, 3).unwrap();
        match result {
            Status::Done(_) => break,
            Status::OutOfFuel => {
                vm.collect().unwrap();
                let image = vm.bytes().to_vec();
                vm = Vm::from_bytes(image, options).unwrap();
            }
            other => panic!("unexpected module status: {other:?}"),
        }
    }
    assert_eq!(
        graph.read_export(&mut vm, "main.js", "count").unwrap(),
        Value::int(9)
    );
    vm.collect_canonical().unwrap();
    let restored = Vm::from_bytes(vm.bytes().to_vec(), options).unwrap();
    vm = restored;
    let read = graph.read_export(&mut vm, "main.js", "read").unwrap();
    vm.call(read, Value::UNDEFINED, &[]).unwrap();
    assert_eq!(vm.run(10000), Status::Done(Value::int(9)));
    assert_eq!(
        evaluate(&graph, &mut vm, "main.js"),
        Status::Done(Value::UNDEFINED)
    );
}

#[test]
fn source_imports_synthetic_host_exports() {
    let mut vm = vm();
    let graph = ModuleGraph::new(&mut vm).unwrap();
    graph.declare(&mut vm, "host:tools").unwrap();
    export(&graph, &mut vm, "host:tools", "value", BindingKind::Mutable);
    graph
        .initialize(&mut vm, "host:tools", "value", Value::int(5))
        .unwrap();
    let mut loader = Sources::new(vec![(
        "main.js",
        "import {value} from 'host:tools'; export function read() { return value; }",
    )]);
    graph.load_source(&mut vm, "main.js", &mut loader).unwrap();
    assert_eq!(
        evaluate(&graph, &mut vm, "main.js"),
        Status::Done(Value::UNDEFINED)
    );
    graph
        .set(&mut vm, "host:tools", "value", Value::int(12))
        .unwrap();
    let read = graph.read_export(&mut vm, "main.js", "read").unwrap();
    vm.call(read, Value::UNDEFINED, &[]).unwrap();
    assert_eq!(vm.run(10000), Status::Done(Value::int(12)));
    assert_eq!(loader.loads, ["main.js"]);
}

#[test]
fn source_cycles_are_explicitly_rejected_without_running_bodies() {
    use djs_core::vm::modules::SourceError;
    let mut vm = vm();
    let graph = ModuleGraph::new(&mut vm).unwrap();
    let mut loader = Sources::new(vec![
        ("a.js", "import './b.js'; export let a = 1;"),
        ("b.js", "import './a.js'; export let b = 2;"),
    ]);
    assert!(matches!(
        graph.load_source(&mut vm, "a.js", &mut loader),
        Err(SourceError::Cycle(_))
    ));
    assert!(graph.begin_evaluation(&mut vm, "a.js").is_err());
}

#[test]
fn source_lexical_scope_hoisting_shadowing_and_destructured_exports() {
    let mut vm = djs_prelude::new_vm(VmOptions::default()).unwrap();
    let graph = ModuleGraph::new(&mut vm).unwrap();
    let mut loader = Sources::new(vec![(
        "main.js",
        r#"
        export const calledEarly = f();
        export const varBefore = value;
        export var value = 4;
        export function f() { return 3; }
        export const {a, b: renamed} = {a: 1, b: 2};
        export let [left, right] = [5, 6];
        export const type = typeof value;
        export const arrowThis = (() => this)();
        { let value = 90; if (value !== 90) throw 'bad block shadow'; }
        export function shadow(value) { return value; }
        export const shadowResult = shadow(7);
        export class C { read() { return value; } }
        export const classResult = new C().read();
    "#,
    )]);
    graph.load_source(&mut vm, "main.js", &mut loader).unwrap();
    assert_eq!(
        evaluate(&graph, &mut vm, "main.js"),
        Status::Done(Value::UNDEFINED)
    );
    for (name, expected) in [
        ("calledEarly", 3),
        ("value", 4),
        ("a", 1),
        ("renamed", 2),
        ("left", 5),
        ("right", 6),
        ("shadowResult", 7),
        ("classResult", 4),
    ] {
        assert_eq!(
            graph.read_export(&mut vm, "main.js", name).unwrap(),
            Value::int(expected),
            "{name}"
        );
    }
    assert_eq!(
        graph.read_export(&mut vm, "main.js", "varBefore").unwrap(),
        Value::UNDEFINED
    );
    assert_eq!(
        graph.read_export(&mut vm, "main.js", "arrowThis").unwrap(),
        Value::UNDEFINED
    );
    let kind = graph.read_export(&mut vm, "main.js", "type").unwrap();
    assert_eq!(vm.heap().str_to_string(kind.as_string().unwrap()), "number");
    let probe = vm
        .compile(
            b"typeof value",
            &djs_core::compiler::CompileOptions::script("global.js"),
        )
        .unwrap();
    vm.call(probe, Value::UNDEFINED, &[]).unwrap();
    let Status::Done(value) = vm.run(1000) else {
        panic!("global isolation probe failed")
    };
    assert_eq!(
        vm.heap().str_to_string(value.as_string().unwrap()),
        "undefined"
    );
}

#[test]
fn source_local_tdz_and_constant_assignment_are_checked() {
    for (source, expected) in [
        ("export const bad = later; let later = 1;", "ReferenceError"),
        (
            "export const bad = typeof later; let later = 1;",
            "ReferenceError",
        ),
        ("const fixed = 1; fixed++;", "TypeError"),
        (
            "const fixed = 1; export function change() { fixed = 2; } change();",
            "TypeError",
        ),
    ] {
        let mut vm = djs_prelude::new_vm(VmOptions::default()).unwrap();
        let graph = ModuleGraph::new(&mut vm).unwrap();
        let mut loader = Sources::new(vec![("main.js", source)]);
        graph.load_source(&mut vm, "main.js", &mut loader).unwrap();
        let Status::Threw(error) = evaluate(&graph, &mut vm, "main.js") else {
            panic!("expected error for {source}")
        };
        assert!(vm.display(error).contains(expected), "{source}");
    }
}

#[test]
fn source_evaluation_parks_on_host_call_and_restores_scheduler() {
    let mut vm = vm();
    let graph = ModuleGraph::new(&mut vm).unwrap();
    graph.declare(&mut vm, "host:tools").unwrap();
    export(&graph, &mut vm, "host:tools", "tool", BindingKind::Constant);
    let function = vm.new_host_function(31, "tool").unwrap();
    graph
        .initialize(&mut vm, "host:tools", "tool", function)
        .unwrap();
    let mut loader = Sources::new(vec![(
        "main.js",
        "import {tool} from 'host:tools'; export const result = tool(6);",
    )]);
    graph.load_source(&mut vm, "main.js", &mut loader).unwrap();
    graph.begin_evaluation(&mut vm, "main.js").unwrap();
    let Status::HostCall { call_id, fn_id } = graph.poll_evaluation(&mut vm, 10000).unwrap() else {
        panic!("module did not park")
    };
    assert_eq!(fn_id, 31);
    assert_eq!(vm.host_call(call_id).unwrap().args, [Value::int(6)]);
    assert!(matches!(
        graph.begin_evaluation(&mut vm, "main.js"),
        Err(djs_core::vm::modules::SourceError::Busy)
    ));
    vm.collect().unwrap();
    let index = graph.handle_index();
    vm = Vm::from_bytes(vm.bytes().to_vec(), vm.options()).unwrap();
    let graph = ModuleGraph::from_handle_index(index);
    vm.resolve_host_call(call_id, Ok(Value::int(18))).unwrap();
    for _ in 0..100 {
        if matches!(graph.poll_evaluation(&mut vm, 10).unwrap(), Status::Done(_)) {
            break;
        }
    }
    assert_eq!(
        graph.read_export(&mut vm, "main.js", "result").unwrap(),
        Value::int(18)
    );
}

#[test]
fn source_default_expression_and_default_binding_have_distinct_liveness() {
    let mut vm = vm();
    let graph = ModuleGraph::new(&mut vm).unwrap();
    let mut loader = Sources::new(vec![
        (
            "dep.js",
            "let value = 1; export default value; export {value as live}; export function inc() { value++; }",
        ),
        (
            "main.js",
            "import captured, {live, inc} from './dep.js'; inc(); export const snapshot = captured; export const current = live;",
        ),
    ]);
    graph.load_source(&mut vm, "main.js", &mut loader).unwrap();
    assert_eq!(
        evaluate(&graph, &mut vm, "main.js"),
        Status::Done(Value::UNDEFINED)
    );
    assert_eq!(
        graph.read_export(&mut vm, "main.js", "snapshot").unwrap(),
        Value::int(1)
    );
    assert_eq!(
        graph.read_export(&mut vm, "main.js", "current").unwrap(),
        Value::int(2)
    );
}

#[test]
fn source_default_declaration_names_are_preserved() {
    for (source, expected) in [
        (
            "export default function answer() {} export const local = answer;",
            "answer",
        ),
        (
            "export default function* answer() {} export const local = answer;",
            "answer",
        ),
        (
            "export default async function answer() {} export const local = answer;",
            "answer",
        ),
        (
            "export default class Answer {} export const local = Answer;",
            "Answer",
        ),
        ("export default function() {}", "default"),
        ("export default class {}", "default"),
    ] {
        let mut vm = djs_prelude::new_vm(VmOptions::default()).unwrap();
        let graph = ModuleGraph::new(&mut vm).unwrap();
        let mut loader = Sources::new(vec![
            ("dep.js", source),
            (
                "main.js",
                "import value from './dep.js'; export const name = value.name;",
            ),
        ]);
        graph.load_source(&mut vm, "main.js", &mut loader).unwrap();
        assert_eq!(
            evaluate(&graph, &mut vm, "main.js"),
            Status::Done(Value::UNDEFINED),
            "{source}"
        );
        let name = graph.read_export(&mut vm, "main.js", "name").unwrap();
        assert_eq!(
            vm.heap().str_to_string(name.as_string().unwrap()),
            expected,
            "{source}"
        );
    }
}

#[test]
fn source_import_asi_does_not_consume_following_assert_call_as_attributes() {
    let mut vm = vm();
    let graph = ModuleGraph::new(&mut vm).unwrap();
    let mut loader = Sources::new(vec![
        ("dep.js", "export const x = 1;"),
        (
            "main.js",
            "function assert(value) { if (!value) throw 'bad'; } import './dep.js'\nassert(true); export const checked = true;",
        ),
    ]);
    graph.load_source(&mut vm, "main.js", &mut loader).unwrap();
    assert_eq!(
        evaluate(&graph, &mut vm, "main.js"),
        Status::Done(Value::UNDEFINED)
    );
    assert_eq!(
        graph.read_export(&mut vm, "main.js", "checked").unwrap(),
        Value::TRUE
    );
}
