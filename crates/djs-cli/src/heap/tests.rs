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
            .at(&["common", "0", "keys"])
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
    // The container holds the session image (D2066), and nothing of the prefix changed.
    let (h, _, _) = container::read_header(&c).unwrap();
    assert_eq!(h.against, container::AGAINST_REFERENCE);
    assert_eq!(json(&r, "prefix").at(&["words"]).num(), 0.0);
}

#[test]
fn the_prefix_section_names_the_changed_builtins() {
    let mut vm = sandbox();
    run_body(
        &mut vm,
        "Array.prototype.extra = 1; Math.max = Math.min; JSON.tag = 'x'; return 1",
    );
    vm.collect().unwrap();
    let r = report(&mut vm);
    let p = json(&r, "prefix");
    assert!(p.at(&["words"]).num() > 0.0);
    let text = super::to_text(&r);
    for name in ["%Array.prototype%", "%Math%", "%JSON%"] {
        assert!(
            text.lines().any(|l| l.trim_end().ends_with(name)),
            "{name} in\n{text}"
        );
    }
}

// ---- A pi session waiting on a tool ------------------------------------------------------

/// scripts/snapshot-research's Rust copy of js/pi/vm.ts's session logic.
#[allow(dead_code, clippy::all)]
#[path = "../../../../scripts/snapshot-research/src/pi.rs"]
mod pi;

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

/// pi's prelude source from tests/pi/prelude-source.ts, evaluated by djs itself (the
/// file is TypeScript with constants interpolated into a template literal).
fn pi_prelude() -> String {
    let ts = std::fs::read_to_string(format!("{ROOT}/tests/pi/prelude-source.ts")).unwrap();
    let body = format!(
        "{}\nreturn PRELUDE_SOURCE;",
        ts.replace("export const", "const")
    );
    let mut vm = vm();
    let f = vm
        .compile(
            body.as_bytes(),
            &CompileOptions::async_body("prelude-source.ts", &[]).ts(),
        )
        .unwrap();
    vm.call(f, Value::UNDEFINED, &[]).unwrap();
    let Status::Done(p) = vm.run(u64::MAX) else {
        panic!("prelude-source.ts did not run")
    };
    let Some(Some(Ok(v))) = vm.promise_state(p) else {
        panic!("no prelude source")
    };
    vm.heap().str_to_string(v.as_string().unwrap())
}

#[test]
fn a_pi_session_waiting_on_a_tool() {
    // As js/pi/vm.ts: the prelude, sealed (the reference image), then the script until it
    // waits; the snapshot collects first.
    let tools = r#"[{"name":"ask","jsName":"ask","description":"Ask."},{"name":"shell","jsName":"shell","description":"Run."}]"#;
    let mut s = pi::Session::prepare(&pi_prelude(), tools, 1_762_000_000_000.0);
    s.vm.seal().unwrap();
    let prelude = s.vm.bytes().to_vec();
    let code =
        std::fs::read_to_string(format!("{ROOT}/scripts/snapshot-research/cases/ask.js")).unwrap();
    s.start(&code, 42);
    assert_eq!(s.ev.calls.len(), 1, "waiting on tools.ask");
    s.vm.collect().unwrap();
    let image = s.vm.bytes().to_vec();
    let r = analyze_image(
        &image,
        Some(("pi's prelude image", &prelude)),
        &Options::default(),
    )
    .unwrap();
    let session = json(&r, "session");
    assert_eq!(
        session.at(&["activations", "0", "kind"]).str(),
        "async function"
    );
    assert_eq!(
        session.at(&["activations", "0", "state"]).str(),
        "at an await"
    );
    assert!(
        session
            .at(&["activations", "0", "at"])
            .str()
            .starts_with("codemode.js:3:"),
        "{:?}",
        session.at(&["activations", "0"])
    );
    assert_eq!(
        session.at(&["activations", "0", "locals", "plan"]).str(),
        "Ordinary"
    );
    assert!(session.at(&["promises", "pending"]).num() >= 1.0);
    // What the session adds to its prelude image: its script and its pending call.
    let d = json(&r, "diff");
    let words = d.at(&["words"]).num();
    assert!(
        d.at(&["same_words"]).num() > 0.9 * words,
        "{:?}",
        d.at(&["same_words"])
    );
    let new = d.at(&["new", "bytes"]).num();
    assert!(new > 1000.0 && new < 16384.0, "{new} new bytes");
    assert!(
        d.at(&["new_subtrees", "0", "path"])
            .str()
            .contains("~pending"),
        "{:?}",
        d.at(&["new_subtrees"])
    );
    assert!(
        d.at(&["gone", "bytes"]).num() < 1024.0,
        "{:?}",
        d.at(&["gone"])
    );
    // The same through a container against the prelude image, given as the reference.
    let c = container::write(
        container::APP_PI_SESSION,
        "{}",
        &image,
        Some((container::REFERENCE_PI_PRELUDE, &prelude)),
        3,
    )
    .unwrap();
    let dir = std::env::temp_dir().join(format!("djs-heap-pi-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (cpath, rpath) = (dir.join("session.djsn"), dir.join("prelude.img"));
    std::fs::write(&cpath, &c).unwrap();
    std::fs::write(&rpath, &prelude).unwrap();
    let missing = super::run(cpath.to_str().unwrap(), None, None, &Options::default())
        .err()
        .unwrap();
    assert!(missing.contains("pi's prelude image"), "{missing}");
    let r2 = super::run(
        cpath.to_str().unwrap(),
        rpath.to_str(),
        None,
        &Options::default(),
    )
    .unwrap();
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(json(&r2, "diff").at(&["new", "bytes"]).num(), new);
    assert_eq!(
        json(&r2, "summary").at(&["container", "reference"]).str(),
        "pi's prelude image"
    );
}

/// D2432: the strings, largest, shapes and functions sections show the session's own
/// part by default, with the reference's part as totals; `--all` the whole image.
#[test]
fn sections_show_the_session_part_unless_all() {
    let mut vm = sandbox();
    let st = run_body(
        &mut vm,
        "globalThis.kept = { label: 'only here' }; return 1",
    );
    assert!(matches!(st, Status::Done(_)), "{st:?}");
    vm.collect().unwrap();
    let base = Some(("the base image", djs_prelude::BASE));
    let own = analyze_image(vm.bytes(), base, &Options::default()).unwrap();
    let all = analyze_image(
        vm.bytes(),
        base,
        &Options {
            all: true,
            ..Options::default()
        },
    )
    .unwrap();
    let (s, a) = (json(&own, "strings"), json(&all, "strings"));
    assert_eq!(s.at(&["scope"]).str(), "session");
    assert_eq!(a.at(&["scope"]).str(), "all");
    let (n, rest, total) = (
        s.at(&["flat"]).num(),
        s.at(&["reference_part", "flat"]).num(),
        a.at(&["flat"]).num(),
    );
    assert!(n > 0.0 && n < 40.0, "{n} session strings");
    assert_eq!(n + rest, total);
    assert!(
        json(&own, "strings")
            .at(&["largest"])
            .arr()
            .iter()
            .any(|x| x.at(&["preview"]).str() == "only here")
    );
    // The base's templates are the reference's part; the session has none of its own
    // closures but the body's.
    let f = json(&own, "functions");
    assert!(f.at(&["templates"]).num() < 5.0);
    assert!(f.at(&["reference_part", "templates"]).num() > 500.0);
    assert!(json(&all, "functions").at(&["templates"]).num() > 500.0);
    let l = json(&own, "largest");
    assert!(l.at(&["largest", "0", "size"]).num() < 8192.0);
}

/// D2431, D2432: a session's atoms go to the tail's own entries; one that adds more than
/// their room regrows that cell, and the diff counts only the words that differ.
#[test]
fn a_regrown_atom_table_differs_only_where_it_changed() {
    let mut vm = sandbox();
    let st = run_body(
        &mut vm,
        "for (let i = 0; i < 40; i++) globalThis['fresh_' + i] = i; return 1",
    );
    assert!(matches!(st, Status::Done(_)), "{st:?}");
    let r = report(&mut vm);
    let changed = json(&r, "diff").at(&["most_changed"]).arr();
    let entries = changed
        .iter()
        .find(|c| c.at(&["path"]).str() == "<atoms><entries>")
        .expect("the session's atom entries changed");
    let (differing, size) = (
        entries.at(&["differing_bytes"]).num(),
        entries.at(&["size"]).num(),
    );
    assert!(differing < size, "{differing} of {size} bytes");
}
