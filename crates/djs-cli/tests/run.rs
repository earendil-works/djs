//! `djs run` with `--tools` and `djs resume` (0174): tool calls answered from a file at
//! idle points, dumps loaded and run to their end.

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
    std::fs::write(&answers, r#"{"glob": [{"value": ["x"]}]}"#).unwrap();
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

/// The format 2 fixtures of the loader (tests/dump/v2, 0171): each `NAME.dump.js` is
/// `NAME.src.js` stopped at an idle point, so resuming it prints the rest of the run.
#[test]
fn resume_the_loader_fixtures() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/dump/v2");
    let mut n = 0;
    for e in std::fs::read_dir(dir).unwrap() {
        let path = e.unwrap().path();
        let p = path.to_str().unwrap();
        let Some(stem) = p.strip_suffix(".dump.js") else {
            continue;
        };
        n += 1;
        let (code, full, err) = djs(&[&format!("{stem}.src.js")]);
        assert_eq!((code, err.as_str()), (0, ""), "{stem}.src.js");
        for extra in [&[][..], &["--suspend-every", "13", "--gc-stress"]] {
            let mut args = vec!["resume"];
            args.extend_from_slice(extra);
            args.push(p);
            let (code, rest, err) = djs_cmd(&args);
            assert_eq!((code, err.as_str()), (0, ""), "{p} {extra:?}");
            assert!(
                !rest.is_empty() && full.ends_with(&rest),
                "{p}:\n{full}---\n{rest}"
            );
        }
    }
    assert!(n >= 5, "{n} fixtures");
}

#[test]
fn resume_with_tools() {
    let src = format!("{DIR}/reads.src.js");
    let dump = format!("{DIR}/reads.dump.js");
    let answers = format!("{DIR}/reads.json");
    let expected = (0, "AABBB\ntotal 5\n".to_string(), String::new());
    assert_eq!(djs(&["--tools", &answers, &src]), expected);
    assert_eq!(djs_cmd(&["resume", "--tools", &answers, &dump]), expected);
    assert_eq!(
        djs_cmd(&["resume", "--tools", &answers, "--suspend-every", "7", &dump]),
        expected
    );
    assert_eq!(
        djs_cmd(&["resume", &dump]),
        (
            1,
            String::new(),
            "djs: no answer for #2 tools.read({\"path\":\"a\"})\n".into()
        )
    );
    let r = djs_cmd(&["resume", &src]);
    assert_eq!(r.0, 2);
    assert!(r.2.contains("not a djs dump"), "{}", r.2);
}

/// `djs resume --explain` (D2641): where a dump's activations stopped, in the dump, since
/// the dump has no comments; it answers nothing.
#[test]
fn explain_a_dump() {
    let dump = format!("{DIR}/reads.dump.js");
    assert_eq!(
        djs_cmd(&["resume", "--explain", &dump]),
        (
            0,
            format!(
                "{dump}: format 2; 1 open call, 1 suspended activation; handles: run\n\
                 #2 tools.read({{\"path\":\"a\"}})\n\
                 \x20 async function main at dump.js:3:13 (run)\n\
                 \x20     const a = await tools.read({{ path: \"a\" }});\n\
                 \x20               ^\n\
                 \x20   a = <uninitialized>, b = <uninitialized>\n"
            ),
            String::new()
        )
    );
    let generator = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/dump/v2/generator.dump.js"
    );
    let (code, out, err) = djs_cmd(&["resume", "--explain", generator]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert!(
        out.contains("not waiting on a call:\n  generator counter at dump.js:"),
        "{out}"
    );
    let r = djs_cmd(&["resume", "--explain", &format!("{DIR}/reads.src.js")]);
    assert_eq!(r.0, 2);
    assert!(r.2.contains("not a djs dump"), "{}", r.2);
}

/// `run --dump-after K` then `resume`, for every idle point K of a script, prints what
/// the uninterrupted run prints; so does `--dump-every-idle`. The number of idle points.
fn every_idle_point(script: &str, args: &[&str]) -> u64 {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let out = format!(
        "{dir}/dump-{}-{}.js",
        std::process::id(),
        script.rsplit('/').next().unwrap()
    );
    let mut plain = args.to_vec();
    plain.push(script);
    let (code, full, err) = djs(&plain);
    assert_eq!((code, err.as_str()), (0, ""), "{script}");
    for k in 1.. {
        let _ = std::fs::remove_file(&out);
        let ks = k.to_string();
        let mut a = args.to_vec();
        a.extend_from_slice(&["--dump-after", &ks, &out, script]);
        let (code, head, err) = djs(&a);
        if !std::path::Path::new(&out).exists() {
            assert_eq!(head, full, "{script}: no dump at {k}");
            assert!(err.contains("the run ended after"), "{script} k={k}: {err}");
            let mut e = args.to_vec();
            e.extend_from_slice(&["--dump-every-idle", script]);
            assert_eq!(
                djs(&e),
                (0, full, String::new()),
                "{script} --dump-every-idle"
            );
            return k - 1;
        }
        assert_eq!((code, err.as_str()), (0, ""), "{script} k={k}");
        let mut r = vec!["resume"];
        r.extend(args.iter().filter(|&&a| a != "--body"));
        r.push(&out);
        let (code, rest, err) = djs_cmd(&r);
        assert_eq!((code, err.as_str()), (0, ""), "{script} k={k} resume");
        assert_eq!(format!("{head}{rest}"), full, "{script} k={k}");
    }
    unreachable!()
}

#[test]
fn dump_after_every_idle_point() {
    let answers = format!("{DIR}/reads.json");
    let n = every_idle_point(&format!("{DIR}/reads.src.js"), &["--tools", &answers]);
    assert_eq!(n, 2);
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/dump/v2");
    let mut total = 0;
    for e in std::fs::read_dir(dir).unwrap() {
        let path = e.unwrap().path();
        let p = path.to_str().unwrap();
        // plan.src.js declares its own `const tools`, which the exporter leaves out as
        // a host global (reported to 0172).
        if !p.ends_with(".src.js") || p.ends_with("/plan.src.js") {
            continue;
        }
        total += every_idle_point(p, &[]);
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
