//! Coverage on the harvested codemode corpus (tests/corpus/codemode-harvest): every case
//! runs with its fake tools waiting on the host, and at its idle points the waiting VM
//! is exported, imported into a fresh VM and finished there; the transcript must equal
//! the uninterrupted run's. Also counts the suspended activations by await site and
//! operand roles, joined with await-sites.mjs's source-level classification.
//!
//! portable harvest [--sources DIR] [--sites SITES.json] [--save DIR] [--max N] CASE...
//! portable import-all DIR        import every saved snapshot and compare transcripts

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Value as J, json};

use crate::export::export;
use crate::import::import;
use crate::session::{Session, case_source};

fn transcript(s: &Session) -> String {
    let mut out = s.output.join("\n");
    out += &format!("\n[{}]", s.finished.clone().unwrap_or_default());
    out
}

#[derive(Default)]
struct Totals {
    cases: u32,
    cases_failed: u32,
    idle: u32,
    exported: u32,
    imported: u32,
    same: u32,
    refusals: BTreeMap<String, u32>,
    activations: u32,
    by_category: BTreeMap<String, u32>,
    by_roles: BTreeMap<String, u32>,
    idle_worst: BTreeMap<String, u32>,
    cases_all_ok: u32,
    sizes: Vec<usize>,
    export_us: Vec<u128>,
    import_us: Vec<u128>,
    image_bytes: Vec<usize>,
}

fn bump(m: &mut BTreeMap<String, u32>, k: &str) {
    *m.entry(k.to_string()).or_default() += 1;
}

fn reason(e: &str) -> String {
    // Group refusals: drop the specifics after the first colon or digit run.
    let e = e.split(" at ").next().unwrap_or(e);
    e.chars().take(60).collect()
}

pub fn main(args: &[String]) {
    let mut sources: Option<String> = None;
    let mut sites_path: Option<String> = None;
    let mut save: Option<String> = None;
    let mut max: usize = 25;
    let mut cases = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--sources" => sources = it.next().cloned(),
            "--sites" => sites_path = it.next().cloned(),
            "--save" => save = it.next().cloned(),
            "--max" => max = it.next().unwrap().parse().unwrap(),
            _ => cases.push(a.clone()),
        }
    }
    let sites: J = sites_path
        .map(|p| serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap())
        .unwrap_or(json!({}));
    let mut t = Totals::default();
    for path in &cases {
        let name = Path::new(path).file_stem().unwrap().to_string_lossy().to_string();
        let text = std::fs::read_to_string(path).unwrap();
        let (src, harness_line) = case_source(&text);
        if let Some(dir) = &sources {
            std::fs::create_dir_all(dir).unwrap();
            std::fs::write(format!("{dir}/{name}.js"), &src).unwrap();
            continue;
        }
        t.cases += 1;
        let Ok(mut whole) = Session::start_case(&text) else {
            t.cases_failed += 1;
            continue;
        };
        let mut n_idle = 0;
        while whole.waiting() {
            n_idle += 1;
            whole.deliver_one();
        }
        whole.run_to_end();
        let want = transcript(&whole);
        if let Some(dir) = &save {
            std::fs::create_dir_all(dir).unwrap();
            std::fs::write(format!("{dir}/{name}.transcript"), &want).unwrap();
        }
        // Idle points to try: all of them, or `max` spread over the run.
        let points: Vec<usize> = if n_idle <= max {
            (0..n_idle).collect()
        } else {
            (0..max).map(|i| i * n_idle / max).collect()
        };
        let mut all_ok = true;
        for k in points {
            let mut s = Session::start_case(&text).unwrap();
            for _ in 0..k {
                s.deliver_one();
            }
            if !s.waiting() {
                continue;
            }
            t.idle += 1;
            let t0 = std::time::Instant::now();
            let exported = export(&mut s);
            let dt = t0.elapsed().as_micros();
            let snap = match exported {
                Ok(j) => j,
                Err(e) => {
                    all_ok = false;
                    let mut seen = std::collections::BTreeSet::new();
                    for r in e {
                        if seen.insert(reason(&r)) {
                            bump(&mut t.refusals, &format!("export: {}", reason(&r)));
                        }
                    }
                    continue;
                }
            };
            t.exported += 1;
            t.export_us.push(dt);
            t.image_bytes.push(s.vm.bytes().len());
            let text_json = serde_json::to_string(&snap).unwrap();
            t.sizes.push(text_json.len());
            if let Some(dir) = &save {
                std::fs::write(format!("{dir}/{name}@{k}.json"), &text_json).unwrap();
            }
            // The suspended activations of the script itself (not the harness).
            let mut worst = "none".to_string();
            for n in snap["nodes"].as_array().unwrap() {
                if n["t"] != "async" || n["fn"]["script"] != "case.js" {
                    continue;
                }
                let line = n["site"]["line"].as_u64().unwrap() as u32;
                if line <= harness_line {
                    continue;
                }
                t.activations += 1;
                let col = n["site"]["col"].as_u64().unwrap();
                let key = format!("{name}:{line}:{col}");
                let cat = sites
                    .get(&key)
                    .and_then(|s| s["cat"].as_str())
                    .unwrap_or("?")
                    .to_string();
                bump(&mut t.by_category, &cat);
                let roles: Vec<&str> = n["operands"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|o| o[0].as_str().unwrap())
                    .collect();
                bump(
                    &mut t.by_roles,
                    &if roles.is_empty() {
                        "(none)".to_string()
                    } else {
                        roles.join(" ")
                    },
                );
                if cat > worst || worst == "none" {
                    worst = cat;
                }
            }
            bump(&mut t.idle_worst, &worst);
            let t0 = std::time::Instant::now();
            let imported = import(&snap);
            t.import_us.push(t0.elapsed().as_micros());
            match imported {
                Ok(mut r) => {
                    t.imported += 1;
                    r.run();
                    r.run_to_end();
                    if transcript(&r) == want {
                        t.same += 1;
                    } else {
                        all_ok = false;
                        if std::env::var("PORTABLE_DEBUG").is_ok() {
                            let got = transcript(&r);
                            let i = got.chars().zip(want.chars()).take_while(|(a, b)| a == b).count();
                            eprintln!("{name}@{k}: differs at {i}:\n got  {:?}\n want {:?}", got.chars().skip(i.saturating_sub(40)).take(160).collect::<String>(), want.chars().skip(i.saturating_sub(40)).take(160).collect::<String>());
                        }
                        bump(&mut t.refusals, "continued: transcript differs");
                    }
                }
                Err(e) => {
                    all_ok = false;
                    bump(&mut t.refusals, &format!("import: {}", reason(&e)));
                }
            }
        }
        if all_ok {
            t.cases_all_ok += 1;
        }
    }
    if sources.is_some() {
        return;
    }
    t.sizes.sort();
    let med = t.sizes.get(t.sizes.len() / 2).copied().unwrap_or(0);
    let max_size = t.sizes.last().copied().unwrap_or(0);
    println!(
        "cases {} (failed to start {}), every tried idle point ok: {}",
        t.cases, t.cases_failed, t.cases_all_ok
    );
    println!(
        "idle points {}: exported {}, imported {}, same transcript {}",
        t.idle, t.exported, t.imported, t.same
    );
    println!("snapshot JSON size: median {med} B, max {max_size} B");
    let med_of = |v: &mut Vec<u128>| {
        v.sort();
        v.get(v.len() / 2).copied().unwrap_or(0)
    };
    t.image_bytes.sort();
    println!(
        "collected image at the same points: median {} B; export median {} us, import median {} us (native dev build, both include building a fresh realm)",
        t.image_bytes.get(t.image_bytes.len() / 2).copied().unwrap_or(0),
        med_of(&mut t.export_us),
        med_of(&mut t.import_us)
    );
    println!("refusals and differences:");
    for (k, v) in &t.refusals {
        println!("  {v:5}  {k}");
    }
    println!("suspended script activations: {}", t.activations);
    println!("  by source category (await-sites.mjs):");
    for (k, v) in &t.by_category {
        println!("    {k}: {v}");
    }
    println!("  by operand roles (this build's bytecode):");
    let mut roles: Vec<_> = t.by_roles.iter().collect();
    roles.sort_by(|a, b| b.1.cmp(a.1));
    for (k, v) in roles.iter().take(15) {
        println!("    {v:5}  {k}");
    }
    println!("  idle points by their worst script activation:");
    for (k, v) in &t.idle_worst {
        println!("    {k}: {v}");
    }
}

/// Imports every snapshot a `harvest --save DIR` run wrote (possibly by another build)
/// and compares the finished transcript with the saved one.
pub fn import_all(dir: &str, also: Option<&str>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    entries.sort();
    let (mut n, mut ok, mut same, mut same_as_importer) = (0, 0, 0, 0);
    let mut refusals: BTreeMap<String, u32> = BTreeMap::new();
    let mut differs = Vec::new();
    for p in entries {
        n += 1;
        let stem = p.file_stem().unwrap().to_string_lossy().to_string();
        let case = stem.split('@').next().unwrap().to_string();
        let want = std::fs::read_to_string(format!("{dir}/{case}.transcript")).unwrap_or_default();
        let snap: J = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
        match import(&snap) {
            Ok(mut r) => {
                ok += 1;
                r.run();
                r.run_to_end();
                let got = transcript(&r);
                let theirs = also
                    .and_then(|d| std::fs::read_to_string(format!("{d}/{case}.transcript")).ok());
                if got == want {
                    same += 1;
                } else if theirs.as_deref() == Some(got.as_str()) {
                    same_as_importer += 1;
                } else {
                    differs.push(stem);
                }
            }
            Err(e) => bump(&mut refusals, &reason(&e)),
        }
    }
    println!(
        "snapshots {n}: imported {ok}; transcript as the exporting build's run: {same}, \
         as this build's own run (where the builds behave differently): {same_as_importer}, \
         neither: {}",
        differs.len()
    );
    for (k, v) in refusals {
        println!("  refused {v:5}  {k}");
    }
    for d in differs.iter().take(20) {
        println!("  differs: {d}");
    }
}
