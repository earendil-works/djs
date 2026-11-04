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

/// A dump (D2830) is a Script: `resume` runs it with the hooks `$djs.resume` and
/// `$djs.seed` (D2890); its open call keeps its id and state.
#[test]
fn resume_with_tools() {
    let src = format!("{DIR}/reads.src.js");
    let dump = format!("{DIR}/reads.dump.js");
    let answers = format!("{DIR}/reads.json");
    let expected = (0, "AABBB\ntotal 5\n".to_string(), String::new());
    assert_eq!(djs(&["--tools", &answers, &src]), expected);
    for extra in [&[][..], &["--suspend-every", "7", "--gc-stress"]] {
        let mut args = vec!["resume", "--tools", &answers];
        args.extend_from_slice(extra);
        args.push(&dump);
        assert_eq!(djs_cmd(&args), expected, "{extra:?}");
    }
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
    // A hook that fails throws in the Script: the dump does not load.
    let dir = env!("CARGO_TARGET_TMPDIR");
    let bad = format!("{dir}/bad-plain-{}.js", std::process::id());
    std::fs::write(&bad, "\"djs-dump\";\n$djs.resume(1, \"nope\", []);\n").unwrap();
    let r = djs_cmd(&["resume", &bad]);
    std::fs::remove_file(&bad).unwrap();
    assert_eq!(
        r,
        (
            2,
            String::new(),
            "djs: the dump did not load: Uncaught Error: resume: unknown host function nope\n"
                .into()
        )
    );
}

/// The CLI's call states (D2894): each open call carries how many ordered answers each
/// tool used, so a resumed run's next call gets the answer the uninterrupted run gets.
#[test]
fn dumps_carry_the_answer_counts() {
    let dir = env!("CARGO_TARGET_TMPDIR");
    let id = std::process::id();
    let (js, answers, dump) = (
        format!("{dir}/counts-{id}.js"),
        format!("{dir}/counts-{id}.json"),
        format!("{dir}/counts-{id}.dump.js"),
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
        r#"{"n": [{"value": 1}, {"value": 2}, {"value": 3}]}"#,
    )
    .unwrap();
    assert_eq!(
        djs(&["--tools", &answers, &js]),
        (0, "1\n2\n3\n".into(), String::new())
    );
    let r = djs(&["--tools", &answers, "--dump-after", "2", &dump, &js]);
    assert_eq!(r, (0, "1\n".into(), String::new()));
    let text = std::fs::read_to_string(&dump).unwrap();
    assert!(
        text.contains(r#", [], {"made":["n"],"answered":{"n":1}});"#),
        "{text}"
    );
    let r = djs_cmd(&["resume", "--tools", &answers, &dump]);
    for f in [&js, &answers, &dump] {
        std::fs::remove_file(f).unwrap();
    }
    assert_eq!(r, (0, "2\n3\n".into(), String::new()));
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
                "{dump}: 1 open call, 1 suspended activation; handles: run\n\
                 #2 tools.read({{\"path\":\"a\"}}), state {{\"at\":\"a\"}}\n\
                 \x20 async function <anonymous> at {dump}:13:12 (run)\n\
                 \x20     const a = (await $dump.p2);\n\
                 \x20                ^\n\
                 \x20   a = undefined, b = undefined\n"
            ),
            String::new()
        )
    );
    // A generator stopped at a yield waits on no call.
    let dir = env!("CARGO_TARGET_TMPDIR");
    let generator = format!("{dir}/explain-generator-{}.js", std::process::id());
    let src = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/dump/v2/generator.src.js"
    );
    let r = djs(&["--dump-after", "1", &generator, src]);
    assert_eq!((r.0, r.2.as_str()), (0, ""));
    let (code, out, err) = djs_cmd(&["resume", "--explain", &generator]);
    std::fs::remove_file(&generator).unwrap();
    assert_eq!((code, err.as_str()), (0, ""));
    assert!(
        out.contains("not waiting on a call:\n  generator "),
        "{out}"
    );
    let r = djs_cmd(&["resume", "--explain", &format!("{DIR}/reads.src.js")]);
    assert_eq!(r.0, 2);
    assert!(r.2.contains("not a djs dump"), "{}", r.2);
}

/// `run --dump-after K` then `resume`, for every idle point K of a script, prints what
/// the uninterrupted run prints; so does `--dump-every-idle`. The number of idle points.
fn every_idle_point(script: &str, args: &[&str]) -> u64 {
    every_idle_point_refused(script, args).0
}

/// [`every_idle_point`], also counting the idle points whose dump is refused (dumps
/// refuse what they cannot write, D2878): (dumped, refused).
fn every_idle_point_refused(script: &str, args: &[&str]) -> (u64, u64) {
    let mut refused = 0;
    let dir = env!("CARGO_TARGET_TMPDIR");
    let out = format!(
        "{dir}/dump-{}-{}{}.js",
        std::process::id(),
        script.rsplit('/').next().unwrap(),
        args.join("").len()
    );
    let mut whole = args.to_vec();
    whole.push(script);
    let (code, full, err) = djs(&whole);
    assert_eq!((code, err.as_str()), (0, ""), "{script}");
    // The hook calls of a dump resumed to its next idle point, which must be
    // the uninterrupted run's at that point: the same call ids, states, Math.random state
    // and next call id (D2897).
    let hooks = |text: &str| -> Vec<String> {
        text.lines()
            .filter_map(|l| {
                l.find("$djs.seed(")
                    .or_else(|| l.find("$djs.resume("))
                    .map(|i| l[i..].to_string())
            })
            .collect()
    };
    let chained = format!("{out}.next.js");
    let mut expected_hooks: Option<Vec<String>> = None;
    for k in 1.. {
        let _ = std::fs::remove_file(&out);
        let ks = k.to_string();
        let mut a = args.to_vec();
        a.extend_from_slice(&["--dump-after", &ks, &out, script]);
        let (code, head, err) = djs(&a);
        if !std::path::Path::new(&out).exists() && err.contains("cannot dump") {
            refused += 1;
            continue;
        }
        if !std::path::Path::new(&out).exists() {
            assert_eq!(head, full, "{script}: no dump at {k}");
            assert!(err.contains("the run ended after"), "{script} k={k}: {err}");
            if refused == 0 {
                let mut e = args.to_vec();
                e.extend_from_slice(&["--dump-every-idle", script]);
                assert_eq!(
                    djs(&e),
                    (0, full, String::new()),
                    "{script} --dump-every-idle"
                );
            }
            return (k - 1 - refused, refused);
        }
        assert_eq!((code, err.as_str()), (0, ""), "{script} k={k}");
        let text = std::fs::read_to_string(&out).unwrap();
        if let Some(want) = expected_hooks.take() {
            assert_eq!(
                hooks(&text),
                want,
                "{script} k={k}: the resumed dump's next dump"
            );
        }
        let mut r = vec!["resume"];
        r.extend(args.iter().filter(|&&a| a != "--body"));
        r.push(&out);
        let (code, rest, err) = djs_cmd(&r);
        assert_eq!((code, err.as_str()), (0, ""), "{script} k={k} resume");
        assert_eq!(format!("{head}{rest}"), full, "{script} k={k}");
        let _ = std::fs::remove_file(&chained);
        let mut r = vec!["resume"];
        r.extend(args.iter().filter(|&&a| a != "--body"));
        r.extend_from_slice(&["--dump-after", "2", &chained, &out]);
        djs_cmd(&r);
        expected_hooks = std::fs::read_to_string(&chained).ok().map(|t| hooks(&t));
    }
    unreachable!()
}

/// `run --dump-after K` (and `--dump-every-idle`) then `resume`, so the CLI's loader
/// runs the exporter's output.
#[test]
fn dump_after_every_idle_point() {
    let answers = format!("{DIR}/reads.json");
    let n = every_idle_point(&format!("{DIR}/reads.src.js"), &["--tools", &answers]);
    assert_eq!(n, 2);
    // A codemode body suspended in its own top level (D2617).
    let dir = env!("CARGO_TARGET_TMPDIR");
    let body = format!("{dir}/dump-body-{}.js", std::process::id());
    std::fs::write(
        &body,
        "const files = await tools.glob({});\n\
         const seen = [];\n\
         for (const f of files) {\n  seen.push((await tools.read({ path: f })).length);\n}\n\
         console.log(seen.join(','));\n",
    )
    .unwrap();
    let answers = format!("{DIR}/todos.json");
    let n = every_idle_point(&body, &["--body", "--tools", &answers]);
    std::fs::remove_file(&body).unwrap();
    assert_eq!(n, 5);
    // A codemode body whose helper closes over the `tools` parameter (D2620).
    let n = every_idle_point(&format!("{DIR}/todos.js"), &["--body", "--tools", &answers]);
    assert!(n >= 2, "{n} idle points");
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/dump/v2");
    let (mut total, mut refused) = (0, 0);
    for e in std::fs::read_dir(dir).unwrap() {
        let path = e.unwrap().path();
        let p = path.to_str().unwrap();
        if p.ends_with(".src.js") {
            let (n, r) = every_idle_point_refused(p, &[]);
            (total, refused) = (total + n, refused + r);
        }
    }
    assert!(total >= 40, "{total} idle points, {refused} refused");
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

/// A dump naming a host function the CLI does not define does not load, not even for
/// `--explain`: the dump's code calls it while it loads.
#[test]
fn an_unknown_host_function() {
    let dump = format!("{DIR}/sleep.dump.js");
    for args in [&["resume", "--explain", &dump][..], &["resume", &dump]] {
        let (code, _, err) = djs_cmd(args);
        assert_eq!(code, 2);
        assert!(err.contains("unknown host function sleep"), "{err}");
    }
}
