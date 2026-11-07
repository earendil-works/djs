//! E3/E4: host functions (sync and async), typed arguments and results, the async entry.

use djs_embed::{Data, Error, ErrorKind, HostFn, Limits, Snapshot, Source, Step, Throw, Value};

const ECHO: u32 = 1;
const READ: u32 = 2;
const FAIL: u32 = 3;

fn snapshot(src: &str) -> Snapshot {
    Snapshot::builder()
        .host_fn(HostFn::sync(ECHO, "echo"))
        .host_fn(HostFn::asynchronous(READ, "read"))
        .host_fn(HostFn::sync(FAIL, "fail"))
        .source(Source::script("t.js", src))
        .build()
        .unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn sync_host_calls_with_typed_arguments() {
    let snap = snapshot(
        "function f() {\n\
           return [echo('ascii'), echo('héllo €'), echo(new Uint8Array([1, 2, 3])), echo(2.5),\n\
                   echo(true), echo(null), echo(), echo({ a: [1, { b: 'c' }] })];\n\
         }",
    );
    let mut i = snap.instantiate(Limits::default()).unwrap();
    let mut seen = Vec::new();
    let out = i
        .call("f", &[], |c| {
            assert_eq!(c.fn_id(), ECHO);
            assert!(!c.is_async());
            let a = c.arg(0);
            let line = if let Some(s) = a.as_str() {
                let borrowed = matches!(s, std::borrow::Cow::Borrowed(_));
                format!("str {s} borrowed={borrowed}")
            } else if let Some(b) = a.as_bytes() {
                format!("bytes {b:?}")
            } else if let Some(n) = a.as_number() {
                format!("number {n}")
            } else if let Some(b) = a.as_bool() {
                format!("bool {b}")
            } else if a.is_null() {
                "null".into()
            } else if a.is_undefined() {
                format!("undefined len={}", c.len())
            } else {
                format!("data {:?}", a.to_data().unwrap())
            };
            seen.push(line.clone());
            c.reply(Ok(Value::Str(&line)))
        })
        .unwrap();
    assert_eq!(
        seen,
        [
            "str ascii borrowed=true",
            "str héllo € borrowed=false",
            "bytes [1, 2, 3]",
            "number 2.5",
            "bool true",
            "null",
            "undefined len=0",
            r#"data Object([("a", Array([Number(1.0), Object([("b", String("c"))])]))])"#,
        ]
    );
    assert_eq!(out.as_array().unwrap().len(), 8);
    assert_eq!(
        out.as_array().unwrap()[1],
        Data::from("str héllo € borrowed=false")
    );
}

#[test]
fn typed_results() {
    let snap = snapshot(
        "function f() {\n\
           const b = echo(0);\n\
           return [typeof echo(1), echo(2), echo(3) instanceof Uint8Array && echo(3).length,\n\
                   echo(4).k[1], b, echo(5), echo(6)];\n\
         }",
    );
    let mut i = snap.instantiate(Limits::default()).unwrap();
    let data = Data::object([("k", Data::Array(vec![Data::Null, Data::from("v")]))]);
    let wire = Data::from("from wire").to_wire();
    let out = i
        .call("f", &[], |c| {
            let n = c.arg(0).as_number().unwrap() as u32;
            let r = match n {
                0 => Value::Bool(true),
                1 => Value::Undefined,
                2 => Value::Number(-0.5),
                3 => Value::Bytes(&[9, 8, 7, 6]),
                4 => Value::Data(&data),
                5 => Value::Wire(&wire),
                _ => Value::Null,
            };
            c.reply(Ok(r))
        })
        .unwrap();
    assert_eq!(
        out,
        Data::Array(vec![
            "undefined".into(),
            (-0.5).into(),
            4.into(),
            "v".into(),
            true.into(),
            "from wire".into(),
            Data::Null,
        ])
    );
}

#[test]
fn host_errors_are_thrown_at_the_call() {
    let snap = snapshot(
        "function f() {\n\
           try { fail(); } catch (e) { return [e instanceof RangeError, e.message]; }\n\
         }\n\
         function g() { fail(); }",
    );
    let mut i = snap.instantiate(Limits::default()).unwrap();
    let out = i
        .call("f", &[], |c| {
            c.reply(Err(Throw::new(ErrorKind::RangeError, "too far")))
        })
        .unwrap();
    assert_eq!(out, Data::Array(vec![true.into(), "too far".into()]));
    let e = i
        .call("g", &[], |c| c.reply(Err(Throw::type_error("bad"))))
        .unwrap_err();
    match e {
        Error::Exception(e) => assert_eq!(e.message, "TypeError: bad"),
        e => panic!("{e:?}"),
    }
}

#[test]
fn async_entry_settles_with_async_host_calls() {
    let snap = snapshot(
        "async function f(name) {\n\
           const [a, b] = await Promise.all([read(name + '1'), read(name + '2')]);\n\
           echo('between');\n\
           const c = await read('3');\n\
           return { a, b, c };\n\
         }",
    );
    let mut i = snap.instantiate(Limits::default()).unwrap();
    i.start("f", &["x".into()]).unwrap();
    let mut waits = Vec::new();
    loop {
        match i.step().unwrap() {
            Step::Done => break,
            Step::Yield => {}
            Step::HostCall(id) => {
                let c = i.host_call(id).unwrap();
                assert_eq!(c.arg(0).as_str().unwrap(), "between");
                c.reply(Ok(Value::Undefined)).unwrap();
            }
            Step::Wait(ids) => {
                waits.push(ids.len());
                // Answered in reverse order: each result keeps its own call.
                for &id in ids.iter().rev() {
                    let c = i.host_call(id).unwrap();
                    assert!(c.is_async());
                    let s = format!("<{}>", c.arg(0).as_str().unwrap());
                    i.resolve(id, Ok(Value::Str(&s))).unwrap();
                }
            }
        }
    }
    assert_eq!(waits, [2, 1]);
    let out = i.result().unwrap().to_data().unwrap();
    assert_eq!(
        out,
        Data::object([
            ("a", Data::from("<x1>")),
            ("b", "<x2>".into()),
            ("c", "<3>".into())
        ])
    );
    // The result stays readable until the next call.
    assert_eq!(i.step().unwrap(), Step::Done);
    assert!(i.fuel_used() > 0);
}

#[test]
fn a_rejected_promise_is_an_exception() {
    let snap = snapshot("async function f() { await read('x'); throw new Error('late'); }");
    let mut i = snap.instantiate(Limits::default()).unwrap();
    let e = i.call("f", &[], |c| c.reply(Ok(Value::Null))).unwrap_err();
    assert!(
        matches!(&e, Error::Exception(x) if x.message == "Error: late"),
        "{e:?}"
    );
    // An async host call answered with an error rejects its promise.
    let snap =
        snapshot("async function f() { try { await read('x'); } catch (e) { return e.message; } }");
    let mut i = snap.instantiate(Limits::default()).unwrap();
    let out = i
        .call("f", &[], |c| c.reply(Err(Throw::error("io"))))
        .unwrap();
    assert_eq!(out, "io".into());
}

#[test]
fn settling_drops_what_the_call_left() {
    // A fire-and-forget async call is dropped when the entry settles.
    let snap = snapshot("function f() { read('ignored'); return 1; }");
    let mut i = snap.instantiate(Limits::default()).unwrap();
    i.start("f", &[]).unwrap();
    assert_eq!(i.step().unwrap(), Step::Done);
    assert_eq!(i.result().unwrap().as_number(), Some(1.0));
    // The next call starts clean.
    assert_eq!(i.call("f", &[], |_| Ok(())).unwrap(), 1.into());
}

#[test]
fn methods_get_their_object_as_this() {
    let snap = snapshot("var hooks = { n: 4, get() { return this.n; } };");
    let mut i = snap.instantiate(Limits::default()).unwrap();
    assert_eq!(i.call("hooks.get", &[], |_| Ok(())).unwrap(), 4.into());
    for bad in ["nope", "hooks.n", "hooks.missing.x"] {
        let e = i.call(bad, &[], |_| Ok(())).unwrap_err();
        assert!(matches!(e, Error::Usage(_)), "{bad}: {e:?}");
    }
}

#[test]
fn one_call_at_a_time() {
    let snap = snapshot("function f() { return echo(); }");
    let mut i = snap.instantiate(Limits::default()).unwrap();
    i.start("f", &[]).unwrap();
    assert!(matches!(i.step().unwrap(), Step::HostCall(_)));
    assert!(matches!(i.start("f", &[]), Err(Error::Usage(_))));
    // An unanswered host call in the driver is misuse.
    i.abort();
    let e = i.call("f", &[], |_| Ok(())).unwrap_err();
    assert!(matches!(e, Error::Usage(_)), "{e:?}");
    // After a failed call the instance serves the next one.
    assert_eq!(
        i.call("f", &[], |c| c.reply(Ok(5.into()))).unwrap(),
        5.into()
    );
}

#[test]
fn a_host_error_in_the_driver_ends_the_call() {
    let snap = snapshot(
        "var after = 0; function f() { echo(); after = 1; }\nfunction get() { return after; }",
    );
    let mut i = snap.instantiate(Limits::default()).unwrap();
    let e = i.call("f", &[], |_| Err(Error::Usage("host gave up".into())));
    assert_eq!(e, Err(Error::Usage("host gave up".into())));
    assert_eq!(i.step(), Err(Error::Usage("the call is over".into())));
    assert_eq!(i.call("get", &[], |_| Ok(())).unwrap(), 0.into());
}
