//! `djs run` with `--tools` and `djs resume` (0174): tool calls answered from a file at
//! idle points, snapshots loaded and run to their end.

use std::process::Command;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/run");

/// Runs `djs run args...`: (exit code, stdout, stderr).
fn djs(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_djs"))
        .arg("run")
        .args(args)
        .output()
        .expect("djs runs");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[test]
fn tools_from_an_answers_file() {
    let js = format!("{DIR}/todos.js");
    let answers = format!("{DIR}/todos.json");
    for extra in [&[][..], &["--suspend-every", "37"], &["--gc-stress"]] {
        let mut args = vec!["--body", "--tools", &answers];
        args.extend_from_slice(extra);
        args.push(&js);
        assert_eq!(
            djs(&args),
            (0, "3 TODOs in 4 files\n".into(), String::new()),
            "{extra:?}"
        );
    }
}

#[test]
fn a_call_without_an_answer() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let answers = format!("{dir}/run-answers-{}.json", std::process::id());
    std::fs::write(&answers, r#"{"tools.glob": [{"value": ["x"]}]}"#).unwrap();
    let r = djs(&["--body", "--tools", &answers, &format!("{DIR}/todos.js")]);
    std::fs::remove_file(&answers).unwrap();
    assert_eq!(
        r,
        (
            1,
            String::new(),
            "djs: no answer for #4 tools.read({\"path\":\"x\"})\n".into()
        )
    );
    let r = djs(&["--tools", "/nonexistent.json", "x.js"]);
    assert_eq!(r.0, 2);
    assert!(
        r.2.starts_with("djs: cannot read /nonexistent.json"),
        "{}",
        r.2
    );
}

/// The CLI's state (D2894): a snapshot carries how many ordered answers each tool used,
/// so a resumed run's next call gets the answer the uninterrupted run gets.
#[test]
fn snapshots_carry_the_answer_counts() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let id = std::process::id();
    let (js, answers, snap) = (
        format!("{dir}/counts-{id}.js"),
        format!("{dir}/counts-{id}.json"),
        format!("{dir}/counts-{id}.snapshot"),
    );
    std::fs::write(
        &js,
        "async function main() { for (let i = 0; i < 3; i++) print(await tools.n()); }
main();
",
    )
    .unwrap();
    std::fs::write(
        &answers,
        r#"{"tools.n": [{"value": 1}, {"value": 2}, {"value": 3}]}"#,
    )
    .unwrap();
    assert_eq!(
        djs(&["--tools", &answers, &js]),
        (0, "1\n2\n3\n".into(), String::new())
    );
    let r = djs(&["--tools", &answers, "--snapshot-after", "2", &snap, &js]);
    assert_eq!(r, (0, "1\n".into(), String::new()));
    let r = djs_cmd(&["resume", "--tools", &answers, &snap]);
    for f in [&js, &answers, &snap] {
        std::fs::remove_file(f).unwrap();
    }
    assert_eq!(r, (0, "2\n3\n".into(), String::new()));
}

/// `run --snapshot-after K` then `resume`, for every idle point K of a script, prints
/// what the uninterrupted run prints; so does `--snapshot-every-idle` (D3720).
fn every_snapshot_point(script: &str, args: &[&str]) -> u64 {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let mut full_args = args.to_vec();
    full_args.push(script);
    let full = djs(&full_args);
    let mut k = 1u64;
    loop {
        let out = format!(
            "{dir}/snap-{}-{}-{}-{k}.snapshot",
            std::process::id(),
            script.rsplit('/').next().unwrap(),
            args.join("").len()
        );
        let ks = k.to_string();
        let mut a = args.to_vec();
        a.extend_from_slice(&["--snapshot-after", &ks, &out, script]);
        let head = djs(&a);
        if !std::path::Path::new(&out).exists() {
            assert_eq!(
                (head.0, &head.1),
                (full.0, &full.1),
                "{script}: no snapshot at {k}"
            );
            break;
        }
        let mut r = vec!["resume"];
        r.extend(resume_args(args));
        r.push(&out);
        let tail = djs_cmd(&r);
        std::fs::remove_file(&out).unwrap();
        assert_eq!(head.0, 0, "{script} k={k}: {head:?}");
        assert_eq!(
            (
                tail.0,
                format!("{}{}", head.1, tail.1),
                format!("{}{}", head.2, tail.2)
            ),
            full,
            "{script} k={k}"
        );
        k += 1;
    }
    let mut e = args.to_vec();
    e.extend_from_slice(&["--snapshot-every-idle", script]);
    assert_eq!(djs(&e), full, "{script} --snapshot-every-idle");
    k - 1
}

/// Every idle point of the CLI's test scripts through a snapshot.
#[test]
fn snapshot_after_every_idle_point() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/run");
    let reads = format!("{dir}/reads.src.js");
    let answers = format!("{dir}/reads.json");
    assert_eq!(every_snapshot_point(&reads, &["--tools", &answers]), 2);
    let tmp = env!("CARGO_TARGET_TMPDIR");
    let body = format!("{tmp}/snap-body-{}.js", std::process::id());
    std::fs::write(
        &body,
        "const a = await tools.read({ path: 'a' });\nconsole.log(a, await later(2));\nconst r = await Promise.all([later(3), tools.read({ path: 'b' })]);\nconsole.log(r);\n",
    )
    .unwrap();
    assert!(every_snapshot_point(&body, &["--body", "--tools", &answers]) >= 3);
    std::fs::remove_file(&body).unwrap();
}

/// `run`'s arguments that `resume` takes too: not `--body` or `--bind NAME=ID` (the
/// snapshot has the body's parameters and the functions bound to them).
fn resume_args<'a>(args: &[&'a str]) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut it = args.iter();
    while let Some(&a) = it.next() {
        match a {
            "--body" => {}
            "--bind" => {
                it.next();
            }
            _ => out.push(a),
        }
    }
    out
}

/// More programs through a snapshot at every idle point: bodies, tools held as values,
/// bound functions and the snapshot test programs.
#[test]
fn snapshot_after_every_idle_point_more() {
    // A codemode body suspended in its own top level (D2617).
    let dir = env!("CARGO_TARGET_TMPDIR");
    let body = format!("{dir}/snap-body2-{}.js", std::process::id());
    std::fs::write(
        &body,
        "const files = await tools.glob({});\n\
         const seen = [];\n\
         for (const f of files) {\n  seen.push((await tools.read({ path: f })).length);\n}\n\
         console.log(seen.join(','));\n",
    )
    .unwrap();
    let answers = format!("{DIR}/todos.json");
    let n = every_snapshot_point(&body, &["--body", "--tools", &answers]);
    std::fs::remove_file(&body).unwrap();
    assert_eq!(n, 5);
    // Tools held as values across idle points: named, given back as the same functions
    // (D2971).
    let held = format!("{dir}/snap-held-{}.js", std::process::id());
    std::fs::write(
        &held,
        "const read = tools.read, glob = tools.glob, t = tools;\n\
         const files = await glob({});\n\
         for (const f of files) console.log((await read({ path: f })).length, read === tools.read, t === tools);\n",
    )
    .unwrap();
    let n = every_snapshot_point(&held, &["--body", "--tools", &answers]);
    std::fs::remove_file(&held).unwrap();
    assert_eq!(n, 5);
    // A function bound as a bare body parameter, not under `tools` (`--bind`, D2972),
    // answered by its identity; held across idle points and compared with itself.
    let bound = format!("{dir}/snap-bound-{}.js", std::process::id());
    let bound_answers = format!("{dir}/snap-bound-{}.json", std::process::id());
    std::fs::write(
        &bound,
        "const held = myTool;\n\
         const a = await myTool(1);\n\
         const b = await held(2);\n\
         console.log(a, b, held === myTool, typeof globalThis.myTool);\n",
    )
    .unwrap();
    std::fs::write(
        &bound_answers,
        r#"{"@sdk/our-stuff#myTool": [{"value": "one"}, {"value": "two"}]}"#,
    )
    .unwrap();
    let bind = [
        "--body",
        "--tools",
        &bound_answers,
        "--bind",
        "myTool=@sdk/our-stuff#myTool",
    ];
    assert_eq!(
        djs(&[&bind[..], &[bound.as_str()]].concat()),
        (0, "one two true undefined\n".into(), String::new())
    );
    let n = every_snapshot_point(&bound, &bind);
    for f in [&bound, &bound_answers] {
        std::fs::remove_file(f).unwrap();
    }
    assert_eq!(n, 2);
    // A codemode body whose helper closes over the `tools` parameter (D2620).
    let n = every_snapshot_point(&format!("{DIR}/todos.js"), &["--body", "--tools", &answers]);
    assert!(n >= 2, "{n} idle points");
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/snapshot/programs");
    let mut total = 0;
    for e in std::fs::read_dir(dir).unwrap() {
        let path = e.unwrap().path();
        let p = path.to_str().unwrap();
        if p.ends_with(".src.js") {
            total += every_snapshot_point(p, &[]);
        }
    }
    assert!(total >= 40, "{total} idle points");
}

/// Runs `djs args...`.
fn djs_cmd(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_djs"))
        .args(args)
        .output()
        .expect("djs runs");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

/// `--debugger OUT` (D3060): the first `debugger;` pause writes the run to OUT as a
/// snapshot (D3720) and stops; `resume --debugger OUT OUT` steps to the next pause,
/// `resume OUT` goes on to the end. Together they print what the run without pauses
/// prints; without the flag `debugger;` does nothing.
#[test]
fn debugger_pauses_snapshot_and_resume() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let id = std::process::id();
    let (js, snap) = (
        format!("{dir}/pause-{id}.js"),
        format!("{dir}/pause-{id}.snapshot"),
    );
    std::fs::write(
        &js,
        "async function main() {
  let total = 0;
  for (let i = 1; i <= 3; i++) {
    total += i;
    console.log('step', i, total);
    debugger;
  }
  return total;
}
main().then((t) => console.log('total', t));
",
    )
    .unwrap();
    let all = "step 1 1\nstep 2 3\nstep 3 6\ntotal 6\n";
    assert_eq!(djs(&[&js]), (0, all.into(), String::new()));
    let r = djs(&["--debugger", &snap, &js]);
    assert_eq!(
        r,
        (
            0,
            "step 1 1\n".into(),
            format!("djs: paused at {js}:6:5, wrote {snap}\n")
        )
    );
    let d = djs_cmd(&["describe", &snap]);
    assert!(d.1.contains(" debugger() "), "{d:?}");
    // The loop's next `debugger;` pauses again, at the same place in the script.
    let r = djs_cmd(&["resume", "--debugger", &snap, &snap]);
    assert_eq!(
        r,
        (
            0,
            "step 2 3\n".into(),
            format!("djs: paused at {js}:6:5, wrote {snap}\n")
        )
    );
    let r = djs_cmd(&["resume", &snap]);
    assert_eq!(r, (0, "step 3 6\ntotal 6\n".into(), String::new()));
    // Every pause, from the start, in --snapshot-every-idle's fresh VMs too.
    let r = djs(&["--snapshot-every-idle", "--debugger", &snap, &js]);
    assert_eq!(r.1, "step 1 1\n");
    for f in [&js, &snap] {
        std::fs::remove_file(f).unwrap();
    }
}

/// A codemode body pauses at its top level; its snapshot holds the body, compiled again
/// with the pause option.
#[test]
fn debugger_pauses_a_body() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let id = std::process::id();
    let (js, snap) = (
        format!("{dir}/pause-body-{id}.js"),
        format!("{dir}/pause-body-{id}.snapshot"),
    );
    std::fs::write(
        &js,
        "debugger;\nconsole.log(await later(1));\ndebugger;\nconsole.log(2);\n",
    )
    .unwrap();
    assert_eq!(djs(&["--body", &js]), (0, "1\n2\n".into(), String::new()));
    let r = djs(&["--body", "--debugger", &snap, &js]);
    assert_eq!(
        r,
        (
            0,
            String::new(),
            format!("djs: paused at {js}:1:1, wrote {snap}\n")
        )
    );
    let r = djs_cmd(&["resume", "--debugger", &snap, &snap]);
    assert_eq!(r.0, 0, "{r:?}");
    assert_eq!(r.1, "1\n");
    assert_eq!(
        djs_cmd(&["resume", &snap]),
        (0, "2\n".into(), String::new())
    );
    for f in [&js, &snap] {
        std::fs::remove_file(f).unwrap();
    }
}
