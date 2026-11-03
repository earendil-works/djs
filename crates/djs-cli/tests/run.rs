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

#[test]
fn dumping_needs_the_exporter() {
    let src = format!("{DIR}/reads.src.js");
    for flag in [&["--dump-after", "1"][..], &["--dump-every-idle"]] {
        let mut args = flag.to_vec();
        args.push(&src);
        let (code, out, err) = djs(&args);
        assert_eq!((code, out.as_str()), (2, ""));
        assert!(
            err.contains("needs the format 2 exporter (issue 0172)"),
            "{err}"
        );
    }
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
