//! The `.heap` report over VMs made here: a sandbox-like VM, one holding data, one waiting
//! on an async host call, a pi session.

use djs_core::compiler::CompileOptions;
use djs_core::value::Value;
use djs_core::vm::{Status, Vm, VmOptions};

use super::{Options, Section, analyze};

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
    analyze(
        vm.bytes(),
        Some(("the base image", djs_prelude::BASE)),
        &Options::default(),
    )
    .unwrap()
}

/// The text of the section whose title starts with `title`.
fn text<'a>(sections: &'a [Section], title: &str) -> &'a str {
    &sections
        .iter()
        .find(|s| s.title.starts_with(title))
        .unwrap_or_else(|| panic!("no section {title}"))
        .text
}

/// The first line of `text` that contains `what`.
fn line<'a>(text: &'a str, what: &str) -> &'a str {
    text.lines()
        .find(|l| l.contains(what))
        .unwrap_or_else(|| panic!("no line with {what:?} in\n{text}"))
}

/// The number right after `before` in `line` (`1.5` in `"image: 1.5 KB"` for `"image: "`).
fn num_after(line: &str, before: &str) -> f64 {
    let rest = &line[line
        .find(before)
        .unwrap_or_else(|| panic!("{before:?} in {line:?}"))
        + before.len()..];
    let n: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    n.parse()
        .unwrap_or_else(|_| panic!("a number after {before:?} in {line:?}"))
}

/// The whitespace-separated columns of a table row.
fn cols(line: &str) -> Vec<&str> {
    line.split_whitespace().collect()
}

/// The rows of a table, up to the next blank line: `heading` is its header line, or a
/// caption (ending in `:`) above the header.
fn rows<'a>(text: &'a str, heading: &str) -> Vec<&'a str> {
    let mut lines = text.lines().skip_while(|l| !l.contains(heading));
    let caption = lines.next().is_some_and(|l| l.ends_with(':'));
    lines
        .skip(usize::from(caption))
        .take_while(|l| !l.trim().is_empty())
        .collect()
}

/// The leading number of the first line that contains `what` and starts with a number
/// (`8` of `"  8 flat strings (0 two-byte), ..."`).
fn count(text: &str, what: &str) -> f64 {
    let l = text
        .lines()
        .find(|l| l.contains(what) && l.trim_start().starts_with(|c: char| c.is_ascii_digit()))
        .unwrap_or_else(|| panic!("no count line with {what:?} in\n{text}"));
    num_after(l, "  ")
}

#[test]
fn a_sandbox_is_the_base_plus_its_console() {
    let mut vm = sandbox();
    let r = report(&mut vm);
    let s = text(&r, "summary");
    let image = num_after(line(s, "image: "), "image: ");
    let live = line(s, "live: ");
    assert_eq!(num_after(live, "garbage: "), 0.0);
    assert_eq!(num_after(live, "a collection leaves "), image);
    let d = text(&r, "diff against");
    let new = cols(line(d, "  new  "));
    let new_kb: f64 = new[2].parse().unwrap();
    assert!(new_kb > 0.0 && new_kb < 4.0, "{new_kb} KB new");
    let subtrees = rows(d, "largest new subtrees");
    assert_eq!(*cols(subtrees[0]).last().unwrap(), "globalThis.console");
    // Most of the image is the base's own cells.
    let kb_of = |what: &str| -> f64 { cols(line(d, what))[3].parse().unwrap() };
    let base_own = kb_of("unchanged, in place") + kb_of("unchanged, moved");
    assert!(base_own > 0.8 * image, "{base_own}");
    assert!(text(&r, "functions").contains("; 4 host functions"));
    // Nothing of the builtin prefix changed.
    assert!(text(&r, "the builtin prefix").contains("no builtin was changed"));
    assert!(s.contains("; 0 words of the builtin prefix changed"));
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
    let subtrees = rows(text(&r, "diff against"), "largest new subtrees");
    let first = cols(subtrees[0]);
    assert_eq!(*first.last().unwrap(), "globalThis.cache");
    assert!(first[1].parse::<u32>().unwrap() >= 600, "{first:?}");
    let common = rows(text(&r, "shapes"), "objects  keys");
    assert_eq!(cols(common[0])[1..], ["id,", "body"]);
    let strings = text(&r, "strings");
    let wasted = line(strings, "contents stored more than once");
    assert!(num_after(wasted, "once, ") > 0.0, "{wasted}");
    let dups = rows(strings, "most wasteful duplicates");
    assert!(
        dups[0].contains(&format!("\"{}…\"", "abc".repeat(20))),
        "{}",
        dups[0]
    );
    let largest = rows(strings, "largest strings");
    assert!(largest[0].ends_with(" globalThis.big"), "{}", largest[0]);
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
    let s = text(&r, "session");
    assert_eq!(cols(rows(s, "host call  function id")[0])[2], "async");
    let act = rows(s, "suspended activations:")[0];
    assert!(act.starts_with("  async function  at an await"), "{act}");
    assert!(act.contains(" case.js:2:11 "), "{act}");
    assert!(num_after(line(s, "promises: "), "promises: ") >= 1.0);
    assert!(s.contains("holds: notes = Array"), "{s}");
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
    let p = text(&r, "the builtin prefix");
    assert!(count(p, " of the builtin prefix ") > 0.0, "{p}");
    let text = super::to_text(&r);
    for name in ["%Array.prototype%", "%Math%", "%JSON%"] {
        assert!(
            text.lines().any(|l| l.trim_end().ends_with(name)),
            "{name} in\n{text}"
        );
    }
}

/// D2432: the strings, largest, shapes and functions sections show the session's own
/// part against a reference, with the reference's part as totals; without one, the whole
/// image.
#[test]
fn sections_show_the_session_part_against_a_reference() {
    let mut vm = sandbox();
    let st = run_body(
        &mut vm,
        "globalThis.kept = { label: 'only here' }; return 1",
    );
    assert!(matches!(st, Status::Done(_)), "{st:?}");
    vm.collect().unwrap();
    let base = Some(("the base image", djs_prelude::BASE));
    let own = analyze(vm.bytes(), base, &Options::default()).unwrap();
    let all = analyze(vm.bytes(), None, &Options::default()).unwrap();
    let (s, a) = (text(&own, "strings"), text(&all, "strings"));
    assert!(s.contains("the session's own part"));
    assert!(!a.contains("the session's own part"));
    let (n, rest, total) = (
        count(s, " flat strings ("),
        num_after(line(s, "unchanged from the reference: "), "reference: "),
        count(a, " flat strings ("),
    );
    assert!(n > 0.0 && n < 40.0, "{n} session strings");
    assert_eq!(n + rest, total);
    assert!(
        rows(s, "largest strings")
            .iter()
            .any(|l| l.contains("\"only here\""))
    );
    // The base's templates are the reference's part; the session has none of its own
    // closures but the body's.
    let f = text(&own, "functions");
    assert!(count(f, " templates, ") < 5.0, "{f}");
    assert!(num_after(line(f, "unchanged from the reference: "), "reference: ") > 500.0);
    assert!(count(text(&all, "functions"), " templates, ") > 500.0);
    let l = rows(text(&own, "largest cells"), "largest cells:");
    assert!(cols(l[0])[0].parse::<f64>().unwrap() < 8.0, "{}", l[0]);
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
    let changed = rows(text(&r, "diff against"), "most changed cells");
    let entries = changed
        .iter()
        .map(|l| cols(l))
        .find(|c| c.last() == Some(&"<atoms><entries>"))
        .expect("the session's atom entries changed");
    let (differing, size): (u32, u32) = (entries[0].parse().unwrap(), entries[1].parse().unwrap());
    assert!(differing < size, "{differing} of {size} bytes");
}
