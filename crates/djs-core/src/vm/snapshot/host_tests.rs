//! Snapshots of a host's run (D3720): host values and functions by name, the handles the
//! host holds by name, its data, and the load into the host's own VM.

extern crate std;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use super::tests::info;
use super::*;
use crate::compiler::CompileOptions;
use crate::heap::Handle;
use crate::interp::Status;
use crate::value::Value;

/// A host's VM: `tools.read` (async) and `print` (sync) under the given ids, the
/// namespace `tools`, and the frozen data `CONFIG`, each a global; the handles of its
/// values by name, and its functions as (identity, id, handle).
struct Host {
    vm: Vm,
    values: Vec<(String, Handle)>,
    functions: Vec<(String, u32, Handle)>,
    printed: Vec<String>,
}

const CONFIG: &str =
    "({ list: [{ name: 'uniqueToolName', schema: { deep: { deeper: 1 } } }], n: 3 })";

fn host(read_id: u32, print_id: u32, with_config: bool) -> Host {
    let mut vm = Vm::from_base(
        super::super::dump_harvest::base(),
        super::super::dump_harvest::frozen(),
    )
    .unwrap();
    let read = vm.new_async_host_function(read_id, "read").unwrap();
    let print = vm.new_host_function(print_id, "print").unwrap();
    let tools = vm.new_null_proto_object().unwrap();
    vm.define(tools, "read", read).unwrap();
    vm.define_global("tools", Value::object(tools)).unwrap();
    vm.define_global("print", print).unwrap();
    let mut values = vec![("tools".to_string(), Value::object(tools))];
    if with_config {
        let src = format!(
            "(() => {{ const deepFreeze = (o) => {{ Object.freeze(o); for (const v of Object.values(o)) if (typeof v === 'object') deepFreeze(v); }}; globalThis.CONFIG = {CONFIG}; deepFreeze(CONFIG); }})();"
        );
        let f = vm
            .compile(src.as_bytes(), &CompileOptions::script("config.js"))
            .unwrap();
        vm.call(f, Value::object(vm.global()), &[]).unwrap();
        assert!(matches!(vm.run(1_000_000), Status::Done(_)));
        let g = vm.global();
        let k = vm.heap_mut().atom_str("CONFIG").unwrap();
        let cfg = vm.heap().get_own_property(g, k).unwrap().value().unwrap();
        values.push(("CONFIG".to_string(), cfg));
    }
    let h = vm.heap_mut();
    let values = values
        .into_iter()
        .map(|(n, v)| (n, h.new_handle(v).unwrap()))
        .collect();
    let functions = vec![
        (
            "tools.read".to_string(),
            read_id,
            h.new_handle(read).unwrap(),
        ),
        ("print".to_string(), print_id, h.new_handle(print).unwrap()),
    ];
    Host {
        vm,
        values,
        functions,
        printed: Vec::new(),
    }
}

impl Host {
    /// Runs until idle or done, printing.
    fn run(&mut self) -> Status {
        loop {
            match self.vm.run(1_000_000_000) {
                Status::HostCall { call_id, .. } => {
                    let args = self.vm.host_call(call_id).unwrap().args;
                    let parts: Vec<String> = args.iter().map(|&v| self.vm.display(v)).collect();
                    self.printed.push(parts.join(" "));
                    self.vm
                        .resolve_host_call(call_id, Ok(Value::UNDEFINED))
                        .unwrap();
                }
                s => return s,
            }
        }
    }

    fn names(&self, handles: Vec<(String, Value)>, data: &str) -> HostNames {
        let h = self.vm.heap();
        HostNames {
            functions: self
                .functions
                .iter()
                .map(|(n, id, _)| (*id, n.clone()))
                .collect(),
            values: self
                .values
                .iter()
                .map(|(n, v)| (n.clone(), h.get_handle(*v)))
                .collect(),
            handles,
            data: data.to_string(),
        }
    }

    fn load(&mut self, bytes: &[u8]) -> Result<Loaded, Refusal> {
        let h = self.vm.heap();
        let functions: Vec<(&str, u32, Value)> = self
            .functions
            .iter()
            .map(|(n, id, v)| (n.as_str(), *id, h.get_handle(*v)))
            .collect();
        let values: Vec<(&str, Value)> = self
            .values
            .iter()
            .map(|(n, v)| (n.as_str(), h.get_handle(*v)))
            .collect();
        let hv = HostValues {
            functions: &functions,
            values: &values,
        };
        self.vm.load_snapshot(info(), bytes, &[], &hv)
    }
}

const BODY: &str = r#"
const first = CONFIG.list[0];
const deep = CONFIG.list[0].schema.deep;
const ns = tools, read = tools.read;
const r = await tools.read("x");
print(first === CONFIG.list[0], deep === CONFIG.list[0].schema.deep, ns === tools, read === tools.read, r);
const s = await read("y");
print("result", r + s);
"#;

/// The run of `BODY` up to its first idle point, and a snapshot of it there.
fn first_snapshot(data: &str) -> (Host, Vec<u8>) {
    let mut a = host(1, 2, true);
    let f =
        a.vm.compile(BODY.as_bytes(), &CompileOptions::async_body("body.js", &[]))
            .unwrap();
    a.vm.call(f, Value::UNDEFINED, &[]).unwrap();
    assert!(matches!(a.run(), Status::Idle));
    let names = a.names(Vec::new(), data);
    let bytes =
        a.vm.snapshot_with(
            info(),
            &SnapshotOptions {
                host: Some(names),
                ..Default::default()
            },
        )
        .unwrap_or_else(|e| panic!("{e:?}"));
    (a, bytes)
}

#[test]
fn a_hosts_run_names_its_values_and_functions() {
    let (_, bytes) = first_snapshot("{\"state\":[1]}");
    // The host's data is not written: its strings are the host's.
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains("uniqueToolName"), "{text}");
    let n = needs(&bytes).unwrap();
    assert_eq!(n.sources, Vec::<String>::new());
    let mut host_names = n.host.clone();
    host_names.sort();
    assert_eq!(host_names, ["CONFIG", "print", "tools", "tools.read"]);

    // Another host, whose function ids differ: the open call is its `tools.read`.
    let mut b = host(7, 8, true);
    let loaded = b.load(&bytes).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(loaded.data, "{\"state\":[1]}");
    assert_eq!(loaded.calls.len(), 1);
    let id = loaded.calls[0];
    assert_eq!(b.vm.host_call(id).unwrap().fn_id, 7);
    let r = b.vm.heap_mut().alloc_str("R").unwrap();
    b.vm.resolve_host_call(id, Ok(Value::string(r))).unwrap();
    assert!(matches!(b.run(), Status::Idle));
    assert_eq!(b.printed, ["true true true true R"]);
    // The next call is new: reported, with the loading host's id.
    let calls = b.vm.new_host_calls();
    assert_eq!(calls.len(), 1);
    assert!(calls[0] > id);
    assert_eq!(b.vm.host_call(calls[0]).unwrap().fn_id, 7);
    let s = b.vm.heap_mut().alloc_str("S").unwrap();
    b.vm.resolve_host_call(calls[0], Ok(Value::string(s)))
        .unwrap();
    assert!(matches!(b.run(), Status::Idle | Status::Done(_)));
    assert!(b.vm.pending_host_calls().is_empty());
    assert_eq!(b.printed[1], "result RS");
}

#[test]
fn handles_travel_by_name() {
    let mut a = host(1, 2, true);
    let f =
        a.vm.compile(
            b"globalThis.kept = { a: 1 };",
            &CompileOptions::script("k.js"),
        )
        .unwrap();
    a.vm.call(f, Value::object(a.vm.global()), &[]).unwrap();
    a.run();
    let g = a.vm.global();
    let k = a.vm.heap_mut().atom_str("kept").unwrap();
    let kept = a.vm.heap().get_own_property(g, k).unwrap().value().unwrap();
    let names = a.names(vec![("mine".to_string(), kept)], "");
    let bytes =
        a.vm.snapshot_with(
            info(),
            &SnapshotOptions {
                host: Some(names),
                ..Default::default()
            },
        )
        .unwrap();
    let mut b = host(1, 2, true);
    let loaded = b.load(&bytes).unwrap();
    assert_eq!(loaded.handles.len(), 1);
    assert_eq!(loaded.handles[0].0, "mine");
    let v = b.vm.heap().get_handle(loaded.handles[0].1);
    let g = b.vm.global();
    let k = b.vm.heap_mut().atom_str("kept").unwrap();
    assert_eq!(
        b.vm.heap().get_own_property(g, k).unwrap().value().unwrap(),
        v
    );
}

#[test]
fn a_missing_host_value_refuses() {
    let (_, bytes) = first_snapshot("");
    let mut b = host(1, 2, false);
    let e = b.load(&bytes).unwrap_err();
    assert!(e.0[0].contains("no host value CONFIG"), "{e:?}");
    // A host's snapshot does not load into a fresh VM, nor a VM's into a host's.
    let Err(e) = Vm::from_snapshot(info(), &bytes) else {
        panic!("loaded")
    };
    assert!(e.0[0].contains("host's run"), "{e:?}");
    let mut a = host(1, 2, true);
    let plain = a.vm.snapshot(info()).unwrap();
    let e = a.load(&plain).unwrap_err();
    assert!(e.0[0].contains("fresh VM"), "{e:?}");
}

#[test]
fn a_host_function_without_a_name_refuses() {
    let mut a = host(1, 2, true);
    let mut names = a.names(Vec::new(), "");
    names.functions.retain(|(id, _)| *id != 2);
    let e =
        a.vm.snapshot_with(
            info(),
            &SnapshotOptions {
                host: Some(names),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert!(e.0[0].contains("host function (id 2)"), "{e:?}");
    let _ = a.run();
}

