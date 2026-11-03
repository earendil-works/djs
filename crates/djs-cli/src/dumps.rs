//! Continuation dumps in the CLI (0174, D2604): writing a waiting session as a format 2
//! dump and loading one, with the host's names for its functions from [`Tools`].

use djs_core::heap::Handle;
use djs_core::value::Value;
use djs_core::vm::Vm;
use djs_core::vm::dump::{DumpOptions, Dumped};
use djs_core::vm::dump_load::{LoadOptions, Loaded, dump_format};

use crate::tools::Tools;

/// The globals the CLI defines in every VM: never dumped, referenced by name.
pub const HOST_GLOBALS: &[&str] = &["console", "tools", "print", "later"];

/// The text is a dump (of any format): the loader's own test, not a prefix match.
pub fn is_dump(text: &[u8]) -> bool {
    std::str::from_utf8(text).is_ok_and(|t| dump_format(t).is_some())
}

/// Dumps the idle VM with the host's `handles`; the refusal's message otherwise.
pub fn dump(vm: &Vm, tools: &Tools, handles: &[(&str, Value)]) -> Result<Dumped, String> {
    dump_names(vm, tools, handles, None)
}

/// [`dump`] of the named globals only (all of them with `None`).
pub fn dump_names(
    vm: &Vm,
    tools: &Tools,
    handles: &[(&str, Value)],
    names: Option<&[&str]>,
) -> Result<Dumped, String> {
    let name = |id: u32| tools.call_name(id);
    let opts = DumpOptions {
        base: djs_prelude::BASE,
        host: HOST_GLOBALS,
        host_fn_name: &name,
        handles,
    };
    vm.dump_report(&opts, names).map_err(|e| e.message())
}

/// Loads a dump into `vm` (a fresh one), the tools named by the registry.
pub fn load(vm: &mut Vm, tools: &Tools, text: &str) -> Result<Loaded, String> {
    let host_fn = |name: &str| tools.id_for_call_name(name);
    let opts = LoadOptions {
        host_fn: &host_fn,
        ..Default::default()
    };
    vm.load_dump(text, &opts).map_err(|e| e.message())
}

/// `(1 activation, 2 pending calls)`, or nothing when nothing is in flight.
pub fn in_flight(d: &Dumped) -> String {
    let (a, c) = (d.activations.len(), d.open_calls);
    if a == 0 && c == 0 {
        return String::new();
    }
    let s = |n: usize| if n == 1 { "" } else { "s" };
    format!(" ({a} activation{}, {c} pending call{})", s(a), s(c))
}

/// Takes a handle by name out of a load's handles.
pub fn take_handle(handles: &mut Vec<(String, Handle)>, name: &str) -> Option<Handle> {
    let i = handles.iter().position(|(n, _)| n == name)?;
    Some(handles.remove(i).1)
}
