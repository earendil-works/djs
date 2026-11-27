//! Logical snapshots in the CLI (D3700, D3720): a waiting run written as a snapshot of the
//! CLI's host, and loaded into a fresh VM of the CLI.
//!
//! The CLI's host values are named, not written: the globals `tools` (the Proxy over the
//! made tools) and `console`, and its host functions by identity (`later`, `print`,
//! `debugger`, `make`, every tool). The tools the run made, the answers file's counts and
//! the REPL's call origins travel as the host's data, JSON:
//! `{"tools": <Tools::call_state>, "origins": [[id, label], ...]}`.

use std::sync::OnceLock;

use djs_core::heap::Handle;
use djs_core::value::Value;
use djs_core::vm::snapshot::{BaseInfo, HostNames, HostValues, SnapshotOptions};
use djs_core::vm::{Vm, VmOptions};

use crate::json::J;
use crate::tools::{DEBUGGER, DEBUGGER_NAME, LATER, MAKE_TOOL, TOOL_BASE, Tools};

/// The snapshots' base: the builtins' paths of the CLI's base image.
fn base() -> &'static BaseInfo {
    static BASE: OnceLock<BaseInfo> = OnceLock::new();
    BASE.get_or_init(|| {
        BaseInfo::new(djs_prelude::BASE, VmOptions::default()).expect("the base loads")
    })
}

/// Whether `bytes` are a snapshot (its magic).
pub fn is_snapshot(bytes: &[u8]) -> bool {
    bytes.starts_with(b"DJSNAP")
}

/// The host functions with fixed ids, by identity.
const FIXED: &[(u32, &str)] = &[
    (LATER, "later"),
    (crate::run::CONSOLE_LOG, "print"),
    (DEBUGGER, DEBUGGER_NAME),
    (MAKE_TOOL, "make"),
];

/// The CLI's host values by name: the globals it defines that are objects.
const VALUES: &[&str] = &["tools", "console"];

/// The waiting run in `vm` as a snapshot: the values in `handles` come back as handles
/// of the loading VM by name; `origin` labels an open call (the REPL's inputs).
pub fn snapshot(
    vm: &mut Vm,
    tools: &Tools,
    handles: &[(&str, Value)],
    origin: &dyn Fn(u32) -> Option<String>,
) -> Result<Vec<u8>, String> {
    let mut functions: Vec<(u32, String)> =
        FIXED.iter().map(|&(id, n)| (id, n.to_string())).collect();
    for id in TOOL_BASE.. {
        let Some(n) = tools.call_name(id) else { break };
        functions.push((id, n));
    }
    let values: Vec<(String, Value)> = VALUES
        .iter()
        .filter_map(|&n| Some((n.to_string(), vm.global_path(n)?)))
        .collect();
    // Compact JSON: the tools' state as `call_state` writes it, and the origins.
    let state = tools
        .call_state(None)
        .unwrap_or_else(|| String::from("null"));
    let origins: Vec<String> = vm
        .pending_host_calls()
        .into_iter()
        .filter_map(|id| Some(format!("[{id},{}]", crate::tools::json_str(&origin(id)?))))
        .collect();
    let data = format!("{{\"tools\":{state},\"origins\":[{}]}}", origins.join(","));
    let host = HostNames {
        functions,
        values,
        handles: handles.iter().map(|&(n, v)| (n.to_string(), v)).collect(),
        data,
    };
    let opts = SnapshotOptions {
        sources: true,
        host: Some(host),
    };
    vm.snapshot_with(base(), &opts)
        .map_err(|e| format!("cannot snapshot the run: {}", e.0.join("; ")))
}

/// What a load gave back: the handles by name, and the open calls' origins.
pub struct Loaded {
    pub handles: Vec<(String, Handle)>,
    pub origins: Vec<(u32, String)>,
}

/// Loads a snapshot of the CLI's run into `vm`, a fresh VM of the CLI (`run::new_vm`):
/// the tools it names are made first, under the ids the registry gives them.
pub fn load(vm: &mut Vm, tools: &mut Tools, bytes: &[u8]) -> Result<Loaded, String> {
    let needs = djs_core::vm::snapshot::needs(bytes).map_err(|e| e.0.join("; "))?;
    for n in &needs.host {
        if !VALUES.contains(&n.as_str()) && !FIXED.iter().any(|(_, f)| f == n) {
            tools.id_for_call_name(n).ok_or_else(|| {
                format!("the snapshot names {n}, which the CLI has no function for")
            })?;
        }
    }
    tools.premake(vm);
    let mut functions: Vec<(String, u32, Value)> = Vec::new();
    // (`make` is held by the `tools` Proxy only, which is not written.)
    for &(id, n) in FIXED {
        if let Some(v) = vm.global_path(n) {
            functions.push((n.to_string(), id, v));
        }
    }
    for id in TOOL_BASE.. {
        let Some(n) = tools.call_name(id) else { break };
        if let Some(v) = tools.host_value(vm, &n) {
            functions.push((n, id, v));
        }
    }
    let mut values: Vec<(&str, Value)> = Vec::new();
    for &n in VALUES {
        if let Some(v) = vm.global_path(n) {
            values.push((n, v));
        }
    }
    let fns: Vec<(&str, u32, Value)> = functions
        .iter()
        .map(|(n, id, v)| (n.as_str(), *id, *v))
        .collect();
    let host = HostValues {
        functions: &fns,
        values: &values,
    };
    let l = vm
        .load_snapshot(base(), bytes, &[], &host)
        .map_err(|e| e.0.join("; "))?;
    let mut origins = Vec::new();
    if let Ok(J::Obj(fields)) = J::parse(&l.data) {
        for (k, v) in fields {
            match (k.as_str(), v) {
                ("tools", s @ J::Obj(_)) => tools.restore_state(&s.render()),
                ("origins", J::Arr(items)) => {
                    for item in items {
                        if let J::Arr(p) = item
                            && let [J::Num(id), J::Str(o)] = p.as_slice()
                        {
                            origins.push((*id as u32, o.clone()));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(Loaded {
        handles: l.handles,
        origins,
    })
}

/// `djs describe FILE`: a snapshot described for people.
pub fn describe_main(args: &[String]) -> i32 {
    let [path] = args else {
        eprintln!("usage: djs describe <file.snapshot>");
        return 2;
    };
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("djs: cannot read {path}: {e}");
            return 2;
        }
    };
    match djs_core::vm::snapshot::describe(&bytes) {
        Ok(t) => {
            print!("{t}");
            0
        }
        Err(e) => {
            eprintln!("djs: {path} is not a snapshot: {}", e.0.join("; "));
            2
        }
    }
}
