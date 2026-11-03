//! The base image by builtin family and by part (0143, docs/empty-heap.md):
//! `cargo run -p djs-prelude --example base-families [-- --tsv] [--touched DIR]`.
//!
//! `--touched DIR`: files of base offsets a run touched, one per case (written by the
//! instrumented build of scripts/size/patches/touch.py): per family, how many cases touch
//! it, and what a lazy realm would materialize per case.
//!
//! Every cell of the base image gets one owner, a builtin *family* (the builtins/*.ts file
//! its code comes from: array, promise, regexp, ...), and one *part* (function objects,
//! templates, bytecode, constant pools, ...):
//! - a function object and its template belong to the family of the template's name in
//!   builtins/gen/prelude.js (each top-level function there follows a `// builtins/x.ts`
//!   comment);
//! - another object belongs to the family most of its function-valued properties
//!   belong to (a prototype, a namespace object), else to the global it hangs off;
//! - every other cell belongs to the first owned cell that reaches it, in that order
//!   (functions, then objects), so code, pools, property storage and constant strings go
//!   with their function or object; what nothing but the realm, the atom table or the
//!   roots reach is "realm" (the atom names among it: "names").

use std::collections::{HashMap, HashSet, VecDeque};

use djs_core::heap::cells::{FuncTemplate, Object};
use djs_core::heap::{Atom, CellKind, Heap, ObjectClass, Property, Ref};
use djs_core::vm::VmOptions;

/// Cells, bytes, bytes per part, functions, functions reachable from globalThis.
type FamilyRow<'a> = (u32, u64, HashMap<&'a str, u64>, u32, u32);

const PRELUDE: &str = include_str!("../../../builtins/gen/prelude.js");

fn main() {
    let tsv = std::env::args().any(|a| a == "--tsv");
    let vm = djs_prelude::new_vm(VmOptions::default()).expect("base");
    let heap: &Heap = vm.heap();
    let a = heap.arena();

    // Function name -> builtins/*.ts file.
    let mut file_of: HashMap<String, String> = HashMap::new();
    let mut current = String::from("other");
    for line in PRELUDE.lines() {
        if let Some(rest) = line.strip_prefix("// builtins/") {
            current = rest.split(".ts").next().unwrap_or("other").to_string();
        } else if let Some(name) = function_name(line) {
            file_of.entry(name).or_insert_with(|| current.clone());
        }
    }

    let cells: Vec<(u32, CellKind, u32)> =
        heap.cells().map(|c| (c.offset, c.kind, c.size)).collect();
    let size: HashMap<u32, u32> = cells.iter().map(|&(o, _, s)| (o, s)).collect();
    let mut owner: HashMap<u32, String> = HashMap::new();
    let mut part: HashMap<u32, &'static str> = HashMap::new();

    // Roles of the cells a template points at.
    let mut template_family: HashMap<u32, String> = HashMap::new();
    for &(o, k, _) in &cells {
        if k != CellKind::FuncTemplate {
            continue;
        }
        let name = heap.atom_name(Atom::from_id(a.get::<u32>(o, FuncTemplate::NAME)));
        let fam = file_of
            .get(&name)
            .cloned()
            .unwrap_or_else(|| "other".into());
        template_family.insert(o, fam);
        // The code, capture descriptors and handlers are inline (D2444).
        part.insert(o, "templates");
        for (f, role) in [
            (FuncTemplate::CONSTS, "constant pools"),
            (FuncTemplate::INFO, "template infos"),
        ] {
            let r: Ref = a.get(o, f);
            if let Some(r) = r.get() {
                part.insert(r, role);
            }
        }
    }

    // Seeds, nearest first: what globalThis reaches (breadth-first, by the global it hangs
    // off), then the installed intrinsics (by the builtins/*.ts file of their name), then
    // everything else by its nearest owner (multi-source breadth-first search over
    // references: helpers captured in boxes go with the builtin that captures them).
    let global = vm.global();
    let mut visible: HashSet<u32> = HashSet::new();
    let mut q: VecDeque<(u32, String)> = VecDeque::new();
    owner.insert(global.offset(), "realm".into());
    for k in heap.own_keys(global) {
        let fam = family_of_global(&heap.atom_name(k));
        for v in values(heap, global, k) {
            if let Some(o) = v.as_object() {
                q.push_back((o.offset(), fam.clone()));
            }
        }
    }
    let mut objects: VecDeque<(u32, String)> = VecDeque::new();
    while let Some((o, fam)) = q.pop_front() {
        if !visible.insert(o) {
            continue;
        }
        owner.entry(o).or_insert_with(|| fam.clone());
        objects.push_back((o, fam.clone()));
        let obj = obj_at(&vm, o);
        if let Some(p) = heap.get_prototype_of(obj) {
            q.push_back((p.offset(), fam.clone()));
        }
        for k in heap.own_keys(obj) {
            for v in values(heap, obj, k) {
                if let Some(x) = v.as_object() {
                    q.push_back((x.offset(), fam.clone()));
                }
            }
        }
    }
    let realm_cell = cells
        .iter()
        .find(|c| c.1 == CellKind::Realm)
        .expect("realm")
        .0;
    let installed: Ref = a.get(realm_cell, djs_core::heap::cells::Realm::INSTALLED);
    for (i, info) in djs_core::bytecode::intrinsics::INTRINSICS
        .iter()
        .enumerate()
    {
        let v: djs_core::value::Value =
            a.read(djs_core::heap::cells::ValueArray::VALUES.at(installed.0, i as u32));
        if let Some(f) = v.as_object()
            && !owner.contains_key(&f.offset())
        {
            let fam = file_of
                .get(info.name)
                .map(|f| family_of_file(f))
                .unwrap_or_else(|| "other".into());
            owner.insert(f.offset(), fam.clone());
            objects.push_back((f.offset(), fam));
        }
    }
    // Multi-source BFS from every owned object.
    let mut q = objects;
    while let Some((c, fam)) = q.pop_front() {
        for r in heap.cell_refs(c) {
            if owner.contains_key(&r) {
                continue;
            }
            owner.insert(r, fam.clone());
            q.push_back((r, fam.clone()));
        }
    }
    for &(o, k, _) in &cells {
        if k == CellKind::Object {
            let class = ObjectClass::from_u32(a.get(o, Object::CLASS));
            part.insert(
                o,
                if class == Some(ObjectClass::Function) {
                    "function objects"
                } else {
                    "other objects"
                },
            );
        }
    }

    // Parts of the rest by kind.
    let atoms: HashSet<u32> = cells
        .iter()
        .filter(|c| c.1 == CellKind::AtomTable)
        .flat_map(|c| heap.cell_refs(c.0))
        .flat_map(|t| heap.cell_refs(t))
        .collect();
    for &(o, k, _) in &cells {
        part.entry(o).or_insert(match k {
            CellKind::PropTable
            | CellKind::PropValues
            | CellKind::Shape
            | CellKind::AccessorPair
            | CellKind::RefArray => "property storage",
            CellKind::Str8 | CellKind::Str16 => "strings",
            CellKind::BoxCell => "closure boxes",
            CellKind::U32Array | CellKind::ValueArray => "arrays (atom table, stacks, indexes)",
            _ => "realm cells",
        });
        owner.entry(o).or_insert_with(|| {
            if atoms.contains(&o) {
                "names (atom strings)".into()
            } else {
                "realm".into()
            }
        });
    }

    if let Some(dir) = std::env::args().skip_while(|a| a != "--touched").nth(1) {
        touched_report(heap, &vm, &cells, &owner, &part, &dir);
        return;
    }

    // Report.
    let parts = [
        "function objects",
        "templates",
        "constant pools",
        "template infos",
        "closure boxes",
        "property storage",
        "strings",
        "other objects",
        "arrays (atom table, stacks, indexes)",
        "realm cells",
    ];
    let mut fam: HashMap<String, FamilyRow> = HashMap::new();
    for &(o, _, s) in &cells {
        let e = fam.entry(owner[&o].clone()).or_default();
        e.0 += 1;
        e.1 += s as u64;
        *e.2.entry(part[&o]).or_default() += s as u64;
        if part[&o] == "function objects" {
            e.3 += 1;
            if visible.contains(&o) {
                e.4 += 1;
            }
        }
    }
    let mut rows: Vec<_> = fam.into_iter().collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.1.1));
    let total: u64 = cells.iter().map(|c| c.2 as u64).sum();
    if tsv {
        print!("family\tcells\tbytes\tfunctions\tvisible");
        for p in parts {
            print!("\t{p}");
        }
        println!();
        for (f, (n, b, ps, fu, vi)) in &rows {
            print!("{f}\t{n}\t{b}\t{fu}\t{vi}");
            for p in parts {
                print!("\t{}", ps.get(p).copied().unwrap_or(0));
            }
            println!();
        }
        return;
    }
    println!("base image: {} cells, {total} bytes\n", cells.len());
    println!(
        "{:<26} {:>6} {:>8} {:>6} {:>5} {:>7} {:>7} {:>7} {:>7} {:>7}",
        "family", "cells", "bytes", "funcs", "vis", "fn-obj", "templ", "pools", "props", "strs"
    );
    for (f, (n, b, ps, fu, vi)) in &rows {
        let g = |p: &str| ps.get(p).copied().unwrap_or(0);
        println!(
            "{f:<26} {n:>6} {b:>8} {fu:>6} {vi:>5} {:>7} {:>7} {:>7} {:>7} {:>7}",
            g("function objects"),
            g("templates"),
            g("constant pools"),
            g("property storage"),
            g("strings"),
        );
    }
    println!("\nby part:");
    let mut by_part: HashMap<&str, (u32, u64)> = HashMap::new();
    for &(o, _, s) in &cells {
        let e = by_part.entry(part[&o]).or_default();
        e.0 += 1;
        e.1 += s as u64;
    }
    for p in parts {
        let (n, b) = by_part.get(p).copied().unwrap_or_default();
        println!("  {p:<38} {n:>6} {b:>8}");
        if p == "templates" {
            let code: u64 = cells
                .iter()
                .filter(|c| c.1 == CellKind::FuncTemplate)
                .map(|c| a.get::<u32>(c.0, FuncTemplate::CODE_LEN) as u64)
                .sum();
            println!("    {:<36} {:>6} {code:>8}", "of which bytecode", "");
        }
    }
    let _ = size;
}

fn values(heap: &Heap, o: djs_core::value::Obj, k: Atom) -> Vec<djs_core::value::Value> {
    match heap.get_own_property(o, k) {
        Some(Property::Data { value, .. }) => vec![value],
        Some(Property::Accessor { getter, setter, .. }) => vec![getter, setter],
        None => vec![],
    }
}

/// A global's family: its own name, with the typed arrays, errors and iterator-ish
/// constructors grouped.
fn family_of_global(name: &str) -> String {
    let n = name;
    if n.ends_with("Array") && n != "Array"
        || n == "ArrayBuffer"
        || n == "SharedArrayBuffer"
        || n == "DataView"
    {
        return "typedarray".into();
    }
    if n.ends_with("Error") {
        return "error".into();
    }
    if n.contains("DisposableStack") {
        return "disposable".into();
    }
    if n == "encodeURIComponent"
        || n == "decodeURIComponent"
        || n == "encodeURI"
        || n == "decodeURI"
        || n == "escape"
        || n == "unescape"
    {
        return "uri".into();
    }
    if matches!(
        n,
        "parseInt" | "parseFloat" | "isNaN" | "isFinite" | "Number"
    ) {
        return "number".into();
    }
    if matches!(n, "WeakMap" | "WeakSet") {
        return "weak".into();
    }
    if matches!(n, "WeakRef" | "FinalizationRegistry") {
        return "weakref".into();
    }
    if matches!(n, "eval" | "Function") {
        return "function".into();
    }
    n.to_lowercase()
}

/// A builtins/*.ts file's family, in the names of family_of_global.
fn family_of_file(f: &str) -> String {
    match f {
        "arraybuffer" | "dataview" => "typedarray".into(),
        "string-regexp" => "regexp".into(),
        "fromasync" => "array".into(),
        "iter" | "iterable" | "spread" => "iteration (shared helpers)".into(),
        "conversion" => "conversion (shared helpers)".into(),
        "asyncgen" | "generator" => "generator".into(),
        "eval" => "function".into(),
        other => other.into(),
    }
}

/// The object at `offset` (the public API builds `Obj`s only from values).
fn obj_at(vm: &djs_core::vm::Vm, offset: u32) -> djs_core::value::Obj {
    let g = djs_core::value::Value::object(vm.global()).to_bits();
    djs_core::value::Value::from_bits((g & !0xFFFF_FFFF) | offset as u64)
        .and_then(|v| v.as_object())
        .expect("an object value")
}

/// The name of a top-level `function NAME(` / `async function NAME(` / `function* NAME(`.
fn function_name(line: &str) -> Option<String> {
    let rest = line
        .strip_prefix("async function")
        .or_else(|| line.strip_prefix("function"))?;
    let rest = rest.trim_start_matches('*').trim_start();
    let end = rest.find('(')?;
    let name = rest[..end].trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// An object cell with its property storage.
fn own_bytes_object(heap: &Heap, cells: &[(u32, CellKind, u32)], o: u32) -> u64 {
    let size = |c: u32| cells.iter().find(|x| x.0 == c).map_or(0, |x| x.2) as u64;
    let mut b = size(o);
    for r in heap.cell_refs(o) {
        if matches!(
            heap.cell_kind(r),
            CellKind::PropTable | CellKind::PropValues | CellKind::AccessorPair
        ) {
            b += size(r);
        }
    }
    b
}

/// Per case: the base functions it called or read, the base objects it read, and what
/// materializing them (and only them) would cost.
fn touched_report(
    heap: &Heap,
    vm: &djs_core::vm::Vm,
    cells: &[(u32, CellKind, u32)],
    owner: &HashMap<u32, String>,
    part: &HashMap<u32, &'static str>,
    dir: &str,
) {
    let a = heap.arena();
    let size: HashMap<u32, u32> = cells.iter().map(|&(o, _, s)| (o, s)).collect();
    // A function's own cells: the object, its template and the template's code cells and
    // constant strings; a plain object's: the object and its property storage.
    let mut own_bytes: HashMap<u32, u64> = HashMap::new();
    let mut fn_of_template: HashMap<u32, u32> = HashMap::new();
    for &(o, k, s) in cells {
        if k != CellKind::Object {
            continue;
        }
        let mut b = s as u64;
        if part[&o] == "function objects" {
            if heap.object_class(obj_at(vm, o)) == ObjectClass::Function {
                let t = heap.closure_template(obj_at(vm, o));
                fn_of_template.insert(t, o);
                b += size[&t] as u64;
                for f in [FuncTemplate::CONSTS, FuncTemplate::INFO] {
                    let r: Ref = a.get(t, f);
                    if let Some(r) = r.get() {
                        b += size[&r] as u64;
                    }
                }
            }
        } else {
            for r in heap.cell_refs(o) {
                if matches!(
                    heap.cell_kind(r),
                    CellKind::PropTable | CellKind::PropValues | CellKind::AccessorPair
                ) {
                    b += size[&r] as u64;
                }
            }
        }
        own_bytes.insert(o, b);
    }
    // Holders: the object whose property holds a function (a prototype, a constructor, a
    // namespace). Object by object materialization creates a holder's property storage
    // and all its function objects, but copies a function's template and code only when
    // it is called (or read and then called).
    let mut holder_of: HashMap<u32, u32> = HashMap::new();
    let mut holder_cost: HashMap<u32, u64> = HashMap::new();
    for &(o, k, _) in cells {
        if k != CellKind::Object {
            continue;
        }
        let obj = obj_at(vm, o);
        let mut cost = own_bytes_object(heap, cells, o);
        for key in heap.own_keys(obj) {
            for v in values(heap, obj, key) {
                if let Some(f) = v.as_object()
                    && part.get(&f.offset()) == Some(&"function objects")
                {
                    holder_of.entry(f.offset()).or_insert(o);
                    cost += size[&f.offset()] as u64;
                }
            }
        }
        holder_cost.insert(o, cost);
    }
    let code_bytes = |f: u32| own_bytes[&f] - size[&f] as u64;
    let mut family_bytes: HashMap<&str, u64> = HashMap::new();
    for &(o, _, s) in cells {
        *family_bytes.entry(owner[&o].as_str()).or_default() += s as u64;
    }
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .expect("touched dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "txt"))
        .collect();
    files.sort();
    let mut fam_cases: HashMap<String, u32> = HashMap::new();
    let mut union_fns: HashSet<u32> = HashSet::new();
    let mut per_case: Vec<(String, usize, usize, u64, u64, u64)> = Vec::new();
    let mut fn_cases: HashMap<u32, u32> = HashMap::new();
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap();
        let offs: HashSet<u32> = text.lines().filter_map(|l| l.parse().ok()).collect();
        let mut fns: HashSet<u32> = HashSet::new();
        let mut objs: HashSet<u32> = HashSet::new();
        for &o in &offs {
            if let Some(&f) = fn_of_template.get(&o) {
                fns.insert(f);
            } else if part.get(&o) == Some(&"function objects") {
                fns.insert(o);
            } else if part.get(&o) == Some(&"other objects") {
                objs.insert(o);
            }
        }
        let fams: HashSet<&str> = fns.iter().chain(&objs).map(|o| owner[o].as_str()).collect();
        for fam in &fams {
            *fam_cases.entry(fam.to_string()).or_default() += 1;
        }
        for &f in &fns {
            *fn_cases.entry(f).or_default() += 1;
        }
        union_fns.extend(&fns);
        let by_fn: u64 = fns.iter().chain(&objs).map(|o| own_bytes[o]).sum();
        let by_family: u64 = fams.iter().map(|f| family_bytes[f]).sum();
        let holders: HashSet<u32> = fns
            .iter()
            .filter_map(|f| holder_of.get(f).copied())
            .chain(objs.iter().copied())
            .collect();
        let by_object: u64 = holders.iter().map(|h| holder_cost[h]).sum::<u64>()
            + fns.iter().map(|&f| code_bytes(f)).sum::<u64>();
        let name = f.file_stem().unwrap().to_string_lossy().into_owned();
        per_case.push((name, fns.len(), objs.len(), by_fn, by_family, by_object));
    }
    let n = files.len() as f64;
    println!(
        "{} runs (the corpus prelude and one case each; `_empty` is the prelude alone)
",
        files.len()
    );
    println!("families by share of runs that touch them:");
    let mut fc: Vec<_> = fam_cases.into_iter().collect();
    fc.sort_by_key(|f| std::cmp::Reverse(f.1));
    for (f, c) in &fc {
        println!(
            "  {f:<28} {:>5.1}%  {:>7} B",
            100.0 * *c as f64 / n,
            family_bytes[f.as_str()]
        );
    }
    let mut fns_sorted: Vec<usize> = per_case.iter().map(|c| c.1).collect();
    fns_sorted.sort();
    let mut by_fn: Vec<u64> = per_case.iter().map(|c| c.3).collect();
    by_fn.sort();
    let mut by_fam: Vec<u64> = per_case.iter().map(|c| c.4).collect();
    by_fam.sort();
    let q = |v: &Vec<u64>, p: f64| v[((v.len() - 1) as f64 * p) as usize];
    let qu = |v: &Vec<usize>, p: f64| v[((v.len() - 1) as f64 * p) as usize];
    println!(
        "
base functions touched per run: median {}, p90 {}, max {}; all runs together {} of 874",
        qu(&fns_sorted, 0.5),
        qu(&fns_sorted, 0.9),
        fns_sorted[fns_sorted.len() - 1],
        union_fns.len()
    );
    println!(
        "materialized per run, function by function: median {} B, p90 {} B, max {} B",
        q(&by_fn, 0.5),
        q(&by_fn, 0.9),
        by_fn[by_fn.len() - 1]
    );
    let mut by_obj: Vec<u64> = per_case.iter().map(|c| c.5).collect();
    by_obj.sort();
    println!(
        "materialized per run, object by object (holders with all their function objects, code of the functions used): median {} B, p90 {} B, max {} B",
        q(&by_obj, 0.5),
        q(&by_obj, 0.9),
        by_obj[by_obj.len() - 1]
    );
    println!(
        "materialized per run, family by family: median {} B, p90 {} B, max {} B",
        q(&by_fam, 0.5),
        q(&by_fam, 0.9),
        by_fam[by_fam.len() - 1]
    );
    if let Some(e) = per_case.iter().find(|c| c.0 == "_empty") {
        println!(
            "the corpus prelude alone: {} functions, {} objects, {} B by function, {} B by object, {} B by family",
            e.1, e.2, e.3, e.5, e.4
        );
    }
    let mut common: Vec<_> = fn_cases
        .into_iter()
        .filter(|(_, c)| *c as f64 >= 0.9 * n)
        .collect();
    common.sort();
    println!("functions touched by >= 90% of runs: {}", common.len());
}
