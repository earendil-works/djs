//! The format 2 loader (0171): hand-written dumps in tests/dump/v2/ and refusals.
//!
//! A fixture is a program `NAME.src.js` and a dump of it `NAME.dump.js`, written by hand
//! at idle point K (its second line: `// at idle point K`). The program prints through
//! the sync host function `print` and waits through the async host function `later(v,
//! rejected)`, which the driver settles at idle points, oldest call first, with `v` (a
//! rejection if `rejected`). The program's output up to idle point K followed by the
//! loaded dump's output must equal the uninterrupted run's.

extern crate std;

use alloc::string::String;
use alloc::vec::Vec;

use super::dump_load::{LoadOptions, Loaded};
use super::*;

const LATER: u32 = 1;
const PRINT: u32 = 2;

fn base() -> &'static [u8] {
    static BASE: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    BASE.get_or_init(|| {
        let mut v = Vm::new(VmOptions::default()).unwrap();
        v.prepare_base_image().unwrap();
        v.bytes().to_vec()
    })
}

/// A fresh VM with `later` and `print`.
fn new_vm() -> Vm {
    let mut vm = Vm::from_base(base(), VmOptions::default()).unwrap();
    let later = vm.new_async_host_function(LATER, "later").unwrap();
    vm.define_global("later", later).unwrap();
    let print = vm.new_host_function(PRINT, "print").unwrap();
    vm.define_global("print", print).unwrap();
    vm
}

fn host_fn(name: &str) -> Option<u32> {
    (name == "later").then_some(LATER)
}

/// Runs the VM to its end, settling `later` calls at idle points (oldest first).
/// `marks[k]` is the number of lines printed when idle point k+1 was reached.
fn drive(vm: &mut Vm, lines: &mut Vec<String>, marks: &mut Vec<usize>) {
    let mut rounds = 0;
    loop {
        rounds += 1;
        assert!(rounds < 10_000, "runaway");
        match vm.run(1_000_000_000) {
            Status::HostCall { call_id, fn_id } => {
                assert_eq!(fn_id, PRINT, "unexpected host call");
                let args = vm.host_call(call_id).unwrap().args;
                let parts: Vec<String> = args.iter().map(|&v| vm.display(v)).collect();
                lines.push(parts.join(" "));
                vm.resolve_host_call(call_id, Ok(Value::UNDEFINED)).unwrap();
            }
            Status::Idle | Status::Done(_) => {
                let Some(&id) = vm.pending_host_calls().iter().min() else {
                    break;
                };
                marks.push(lines.len());
                settle(vm, id);
            }
            Status::Threw(e) => {
                let m = vm.display(e);
                lines.push(alloc::format!("threw {m}"));
                break;
            }
            s => panic!("{s:?}"),
        }
    }
    for r in vm.take_unhandled_rejections() {
        let m = vm.display(r);
        lines.push(alloc::format!("unhandled {m}"));
    }
}

/// Settles the `later` call `id` with its arguments.
fn settle(vm: &mut Vm, id: u32) {
    let info = vm.host_call(id).unwrap();
    assert_eq!(info.fn_id, LATER);
    let v = info.args.first().copied().unwrap_or(Value::UNDEFINED);
    let rejected = info.args.get(1).copied() == Some(Value::TRUE);
    vm.resolve_host_call(id, if rejected { Err(v) } else { Ok(v) })
        .unwrap();
}

/// The uninterrupted run of a program: its lines and idle marks.
fn run_program(src: &str) -> (Vec<String>, Vec<usize>) {
    let mut vm = new_vm();
    let f = vm
        .compile(
            src.as_bytes(),
            &crate::compiler::CompileOptions::script("src.js"),
        )
        .unwrap_or_else(|e| panic!("{} at {}:{}", e.message, e.line, e.col));
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    let (mut lines, mut marks) = (Vec::new(), Vec::new());
    drive(&mut vm, &mut lines, &mut marks);
    (lines, marks)
}

fn load(vm: &mut Vm, text: &str) -> Result<Loaded, String> {
    let opts = LoadOptions {
        host_fn: &host_fn,
        ..LoadOptions::default()
    };
    vm.load_dump(text, &opts).map_err(|e| e.message())
}

/// Loads `dump` and runs it to the end: the lines it prints.
fn continue_dump(text: &str) -> (Loaded, Vec<String>) {
    let mut vm = new_vm();
    let loaded = load(&mut vm, text).unwrap_or_else(|e| panic!("{e}"));
    let (mut lines, mut marks) = (Vec::new(), Vec::new());
    drive(&mut vm, &mut lines, &mut marks);
    for (_, h) in &loaded.handles {
        vm.heap_mut().drop_handle(*h);
    }
    (loaded, lines)
}

fn fixture_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/dump/v2")
}

/// Checks fixture `name`; returns what the load reported.
fn fixture(name: &str) -> Loaded {
    let dir = fixture_dir();
    let src = std::fs::read_to_string(dir.join(alloc::format!("{name}.src.js"))).unwrap();
    let dump = std::fs::read_to_string(dir.join(alloc::format!("{name}.dump.js"))).unwrap();
    assert!(dump.starts_with("// djs-dump 2"), "{name}: header");
    let k: usize = dump
        .lines()
        .nth(1)
        .and_then(|l| l.strip_prefix("// at idle point "))
        .and_then(|l| l.split(|c: char| !c.is_ascii_digit()).next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("{name}: no `// at idle point K` line"));
    let (want, marks) = run_program(&src);
    assert!(
        k >= 1 && k <= marks.len(),
        "{name}: {} idle points",
        marks.len()
    );
    let (loaded, after) = continue_dump(&dump);
    let mut got: Vec<String> = want[..marks[k - 1]].to_vec();
    got.extend(after);
    assert_eq!(got, want, "{name}");
    if std::env::var_os("DJS_DUMP_SHOW").is_some() {
        std::println!(
            "{name}: idle point {k} of {}, {} lines after it:",
            marks.len(),
            want.len() - marks[k - 1]
        );
        for l in &want {
            std::println!("  {l}");
        }
    }
    loaded
}

#[test]
fn plan_example() {
    let l = fixture("plan");
    assert_eq!(
        l.calls,
        [super::dump_load::OpenCall {
            id: 5,
            fn_id: LATER,
            name: "later".into()
        }]
    );
    assert_eq!(l.handles.len(), 1);
}

#[test]
fn every_operand_role() {
    let l = fixture("roles");
    assert_eq!(l.calls.len(), 17);
}

#[test]
fn generator_at_a_yield() {
    fixture("generator");
}

#[test]
fn async_generators() {
    fixture("asyncgen");
}

/// Prints the suspension maps of a file, for writing fixtures by hand:
/// `DJS_SMAP=tests/dump/v2/x.src.js cargo test -p djs-core --lib show_maps -- --ignored --nocapture`.
#[test]
#[ignore]
fn show_maps() {
    let path = std::env::var("DJS_SMAP").expect("DJS_SMAP=file");
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(path),
    )
    .unwrap();
    let mut vm = new_vm();
    let maps = crate::compiler::suspension_maps(
        vm.heap_mut(),
        src.as_bytes(),
        &crate::compiler::CompileOptions::script("src.js"),
    )
    .unwrap();
    for m in maps {
        std::print!("{}", m.text(src.as_bytes()));
    }
}
