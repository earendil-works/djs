//! `djs heap` over images made here: a sandbox-like VM, one holding data, one waiting on
//! an async host call, and the same image in a snapshot container.

use djs_core::compiler::CompileOptions;
use djs_core::value::Value;
use djs_core::vm::{Status, Vm, VmOptions};

use super::json::J;
use super::{Options, Section, analyze_image};
use crate::container;

fn vm() -> Vm {
    djs_prelude::new_vm(VmOptions::default()).unwrap()
}

/// What the Sandbox makes: a `console` of four host functions.
fn sandbox() -> Vm {
    let mut vm = vm();
    let g = vm.global();
    let console = vm.new_object().unwrap();
    for (i, m) in ["log", "info", "warn", "error"].iter().enumerate() {
        let f = vm
            .new_host_function(i as u32 + 1, &format!("console.{m}"))
            .unwrap();
        vm.define(console, m, f).unwrap();
    }
    vm.define(g, "console", Value::object(console)).unwrap();
    vm
}

fn run_body(vm: &mut Vm, src: &str) -> Status {
    let f = vm
        .compile(src.as_bytes(), &CompileOptions::async_body("case.js", &[]))
        .unwrap();
    let this = Value::object(vm.global());
    vm.call(f, this, &[]).unwrap();
    vm.run(u64::MAX)
}

fn report(vm: &mut Vm) -> Vec<Section> {
    vm.collect().unwrap();
    analyze_image(
        vm.bytes(),
        Some(("the base image", djs_prelude::BASE)),
        &Options::default(),
    )
    .unwrap()
}

fn json<'a>(sections: &'a [Section], key: &str) -> &'a J {
    &sections.iter().find(|s| s.key == key).unwrap().json
}

#[test]
fn a_sandbox_is_the_base_plus_its_console() {
    let mut vm = sandbox();
    let r = report(&mut vm);
    let s = json(&r, "summary");
    assert_eq!(s.at(&["garbage_bytes"]).num(), 0.0);
    assert_eq!(
        s.at(&["image_bytes"]).num(),
        s.at(&["collected_bytes"]).num()
    );
    let d = json(&r, "diff");
    let new = d.at(&["new", "bytes"]).num();
    assert!(new > 0.0 && new < 4096.0, "{new} new bytes");
    assert_eq!(
        d.at(&["new_subtrees", "0", "path"]).str(),
        "globalThis.console"
    );
    // Most of the image is the base's own cells.
    let base_own = d.at(&["in_place", "bytes"]).num() + d.at(&["moved", "bytes"]).num();
    assert!(base_own > 0.8 * s.at(&["image_bytes"]).num(), "{base_own}");
    assert_eq!(json(&r, "functions").at(&["host_functions"]).num(), 4.0);
}

#[test]
fn data_shows_up_with_its_path_strings_and_shape() {
    let mut vm = sandbox();
    let st = run_body(
        &mut vm,
        "globalThis.cache = Array.from({ length: 200 }, (_, i) => ({ id: i, body: 'item number ' + i })); \
         globalThis.twice = ['abc'.repeat(300), 'abc'.repeat(300)]; globalThis.big = Array(3000).fill('xy').join('');",
    );
    assert!(matches!(st, Status::Done(_)), "{st:?}");
    let r = report(&mut vm);
    let d = json(&r, "diff");
    assert_eq!(
        d.at(&["new_subtrees", "0", "path"]).str(),
        "globalThis.cache"
    );
    assert!(d.at(&["new_subtrees", "0", "cells"]).num() >= 600.0);
    assert_eq!(
        json(&r, "shapes")
            .at(&["common", "1", "keys"])
            .at(&["0"])
            .str(),
        "id"
    );
    let strings = json(&r, "strings");
    assert!(strings.at(&["duplicate_bytes"]).num() > 0.0);
    assert_eq!(
        strings.at(&["duplicates", "0", "string"]).str(),
        "abc".repeat(300).chars().take(200).collect::<String>()
    );
    assert_eq!(
        strings.at(&["largest", "0", "path"]).str(),
        "globalThis.big"
    );
}

#[test]
fn a_waiting_vm_shows_its_suspended_function_and_pending_call() {
    let mut vm = sandbox();
    let g = vm.global();
    let tool = vm.new_async_host_function(9, "tool").unwrap();
    vm.define(g, "tool", tool).unwrap();
    let st = run_body(
        &mut vm,
        "const notes = ['a', 'b'];\nconst r = await tool(notes.length);\nreturn r + notes.length;",
    );
    assert_eq!(st, Status::Idle);
    let r = report(&mut vm);
    let s = json(&r, "session");
    assert_eq!(s.at(&["host_calls", "0", "async"]), &J::Bool(true));
    assert_eq!(s.at(&["activations", "0", "kind"]).str(), "async function");
    assert_eq!(s.at(&["activations", "0", "state"]).str(), "at an await");
    assert_eq!(s.at(&["activations", "0", "at"]).str(), "case.js:2:11");
    assert!(s.at(&["promises", "pending"]).num() >= 1.0);
    assert_eq!(
        s.at(&["activations", "0", "locals", "notes"]).str(),
        "Array"
    );
}

#[test]
fn a_container_reads_like_its_image() {
    let mut vm = sandbox();
    vm.collect().unwrap();
    let image = vm.bytes().to_vec();
    let c = container::write(
        container::APP_SANDBOX,
        djs_core::vm::BUILD_HASH,
        "{}",
        &image,
        Some((container::REFERENCE_BASE, djs_prelude::BASE)),
        container::DEFAULT_LEVEL,
    )
    .unwrap();
    let dir = std::env::temp_dir().join(format!("djs-heap-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("sandbox.djsn");
    std::fs::write(&path, &c).unwrap();
    let r = super::run(path.to_str().unwrap(), None, None, &Options::default()).unwrap();
    std::fs::remove_dir_all(&dir).ok();
    let s = json(&r, "summary");
    assert_eq!(s.at(&["image_bytes"]).num(), image.len() as f64);
    assert_eq!(s.at(&["container", "reference"]).str(), "the base image");
    assert_eq!(
        json(&r, "diff").at(&["new_subtrees", "0", "path"]).str(),
        "globalThis.console"
    );
    let all = super::to_json(&r);
    assert!(all.contains("\"summary\"") && all.contains("\"session\""));
}
