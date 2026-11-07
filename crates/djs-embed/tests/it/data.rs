//! E5: plain data in and out without JSON.

use djs_embed::{Data, Error, Value};

use crate::{call, instance};

#[test]
fn data_round_trips_through_the_vm() {
    let mut i = instance("function id(x) { return x; }");
    let v = Data::object([
        ("s", Data::from("héllo 😀")),
        ("n", Data::from(-1.25)),
        ("i", 7.into()),
        ("b", Data::Bytes(vec![0, 255])),
        (
            "a",
            Data::Array(vec![Data::Null, Data::Hole, Data::Undefined, true.into()]),
        ),
        (
            "o",
            Data::object([("deep", Data::object([("x", Data::from("y"))]))]),
        ),
        ("e", Data::Object(vec![])),
    ]);
    let out = i.call("id", &[Value::Data(&v)], |_| Ok(())).unwrap();
    assert_eq!(out, v);
    // The same through wire bytes.
    let wire = v.to_wire();
    let out = i.call("id", &[Value::Wire(&wire)], |_| Ok(())).unwrap();
    assert_eq!(out, v);
    assert_eq!(Data::from_wire(&wire).unwrap(), v);
}

/// `Value::Data` builds in the heap directly: the same objects the wire form decodes to.
#[test]
fn data_in_is_what_its_wire_form_decodes_to() {
    let mut i = instance(
        "function f(x) {\n\
           return [Object.getPrototypeOf(x) === Object.prototype, Array.isArray(x.a),\n\
             1 in x.a, x.a.length, Object.keys(x).join(), x.b instanceof Uint8Array,\n\
             x.b.length, x.s.length, typeof x.i, Object.hasOwn(x, '__proto__')];\n\
         }\n\
         function id(x) { return x; }",
    );
    let v = Data::Object(vec![
        (
            "a".into(),
            Data::Array(vec![1.into(), Data::Hole, 3.into()]),
        ),
        ("10".into(), "ten".into()),
        ("b".into(), Data::Bytes(vec![1, 2, 3])),
        ("s".into(), "hé😀".into()),
        ("i".into(), 7.into()),
        ("__proto__".into(), Data::Null),
    ]);
    let direct = i.call("f", &[Value::Data(&v)], |_| Ok(())).unwrap();
    let wire = v.to_wire();
    let decoded = i.call("f", &[Value::Wire(&wire)], |_| Ok(())).unwrap();
    assert_eq!(direct, decoded);
    let expected: Vec<Data> = vec![
        true.into(),
        true.into(),
        false.into(),
        3.into(),
        "10,a,b,s,i,__proto__".into(),
        true.into(),
        3.into(),
        4.into(),
        "number".into(),
        true.into(),
    ];
    assert_eq!(direct, Data::Array(expected));
    // The same from borrowed parts (no holes: a `Value` has none).
    let items = [1.into(), Value::Data(&Data::Undefined), 3.into()];
    let fields = [
        ("a", Value::Array(&items)),
        ("10", "ten".into()),
        ("b", Value::Bytes(&[1, 2, 3])),
        ("s", "hé😀".into()),
        ("i", 7.into()),
        ("__proto__", Value::Null),
    ];
    let borrowed = i.call("f", &[Value::Object(&fields)], |_| Ok(())).unwrap();
    let Data::Array(mut b) = borrowed else {
        panic!("{borrowed:?}")
    };
    assert_eq!(b[2], true.into()); // `1 in x.a`: undefined, not a hole
    b[2] = false.into();
    assert_eq!(Data::Array(b), direct);
    let e = Value::Array(&[]);
    let nested = [("e", e), ("o", Value::Object(&[]))];
    let out = i
        .call("id", &[Value::Object(&nested)], |_| Ok(()))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        out,
        Data::object([("e", Data::Array(vec![])), ("o", Data::Object(vec![]))])
    );
    // What the decoder refuses, the direct build refuses too.
    for bad in [Data::Hole, Data::object([("h", Data::Hole)])] {
        let e = i.call("f", &[Value::Data(&bad)], |_| Ok(())).unwrap_err();
        assert!(matches!(e, Error::Data(_)), "{e:?}");
        assert!(
            i.call("f", &[Value::Wire(&bad.to_wire())], |_| Ok(()))
                .is_err()
        );
    }
    let mut deep = Data::Null;
    for _ in 0..6000 {
        deep = Data::Array(vec![deep]);
    }
    let e = i.call("f", &[Value::Data(&deep)], |_| Ok(())).unwrap_err();
    assert!(matches!(e, Error::Data(_)), "{e:?}");
    // (Dropping a 6000-deep tree recurses on the test thread's stack: unwrap it first.)
    while let Data::Array(mut a) = deep {
        deep = a.pop().unwrap();
    }
}

#[test]
fn outbound_data_follows_js() {
    let mut i = instance(
        "function f() {\n\
           const o = { b: 1, a: 2, 10: 'ten', 2: 'two' };\n\
           Object.defineProperty(o, 'hidden', { value: 1, enumerable: false });\n\
           o[Symbol('s')] = 1;\n\
           const a = [1, , 3]; a.extra = 'ignored';\n\
           return { o, a, u: undefined, nan: NaN, neg: -0, lone: '\\ud800x', nul: Object.create(null) };\n\
         }",
    );
    let out = call(&mut i, "f").unwrap();
    let o = out.get("o").unwrap().as_object().unwrap();
    let keys: Vec<&str> = o.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(keys, ["2", "10", "b", "a"]);
    assert_eq!(
        out.get("a").unwrap(),
        &Data::Array(vec![1.into(), Data::Hole, 3.into()])
    );
    assert_eq!(out.get("u"), Some(&Data::Undefined));
    assert!(out.get("nan").unwrap().as_f64().unwrap().is_nan());
    assert!(out.get("neg").unwrap().as_f64().unwrap().is_sign_negative());
    assert_eq!(out.get("lone").unwrap().as_str(), Some("\u{FFFD}x"));
    assert_eq!(out.get("nul"), Some(&Data::Object(vec![])));
    // The wire form keeps the lone surrogate exactly.
    i.start("f", &[]).unwrap();
    assert_eq!(i.step().unwrap(), djs_embed::Step::Done);
    let wire = i.result().unwrap().to_wire().unwrap();
    let back = instance("function id(x) { return x.lone.charCodeAt(0); }")
        .call("id", &[Value::Wire(&wire)], |_| Ok(()))
        .unwrap();
    assert_eq!(back, 0xD800.into());
}

#[test]
fn outbound_refuses_what_is_not_plain_data() {
    let cases = [
        ("() => 1", "cannot encode a function at $"),
        (
            "({ a: [1, { f() {} }] }).a",
            "cannot encode a function at $[1].f",
        ),
        (
            "({ get x() { return 1; } })",
            "cannot encode an accessor at $.x",
        ),
        (
            "(() => { const a = [0]; a.push(a); return a; })()",
            "cyclic value at $[1]",
        ),
        (
            "(() => { const o = {}; o.self = o; return { o }; })()",
            "cyclic value at $.o.self",
        ),
        (
            "new (class P { constructor() { this.x = 1; } })()",
            "cannot encode an object with a custom prototype (a class instance) at $",
        ),
        (
            "({ 'a b': new Map() })",
            "cannot encode a Map at $[\"a b\"]",
        ),
        ("({ d: new Date(0) })", "cannot encode a Date at $.d"),
        ("new Proxy({}, {})", "cannot encode a Proxy at $"),
        ("[Symbol()]", "cannot encode a symbol at $[0]"),
        ("({ n: 10n })", "cannot encode a BigInt at $.n"),
        (
            "new Uint16Array(2)",
            "cannot encode a typed array other than Uint8Array at $",
        ),
    ];
    for (expr, msg) in cases {
        let mut i = instance(&format!("function f() {{ return {expr}; }}"));
        match call(&mut i, "f") {
            Err(Error::Data(m)) => assert_eq!(m, msg, "{expr}"),
            r => panic!("{expr}: {r:?}"),
        }
    }
    // An exponential sharing blowup is bounded by the memory limit, as a tree and as wire.
    let mut i = crate::snapshot(
        "function f() { let v = [1]; for (let i = 0; i < 40; i++) v = [v, v]; return v; }",
    )
    .instantiate(djs_embed::Limits {
        memory: 8 << 20,
        ..Default::default()
    })
    .unwrap();
    assert!(matches!(call(&mut i, "f"), Err(Error::Data(m)) if m.contains("exceeds")));
    i.start("f", &[]).unwrap();
    assert_eq!(i.step().unwrap(), djs_embed::Step::Done);
    let e = i.result().unwrap().to_wire().unwrap_err();
    assert!(matches!(e, Error::Data(m) if m.contains("exceeds")));
}

#[test]
fn corrupt_wire_input_is_a_data_error() {
    let mut i = instance("function id(x) { return x; }");
    for bad in [
        &b"DJW"[..],
        b"DJW\x01\x99",
        b"DJW\x01\x10\x05\x05ab",
        b"XYZ\x01\x00",
    ] {
        let e = i.call("id", &[Value::Wire(bad)], |_| Ok(())).unwrap_err();
        assert!(matches!(e, Error::Data(_)), "{bad:?}: {e:?}");
        assert!(Data::from_wire(bad).is_err());
    }
}

#[test]
fn large_strings_and_bytes() {
    let mut i =
        instance("function f(s, b) { return [s.length, b.length, s.slice(-3), b[b.length - 1]]; }");
    let s = "a".repeat(1 << 16) + "é€😀";
    let b = vec![7u8; 1 << 16];
    let out = i
        .call("f", &[Value::Str(&s), Value::Bytes(&b)], |_| Ok(()))
        .unwrap();
    assert_eq!(
        out,
        Data::Array(vec![
            ((1 << 16) + 4).into(),
            (1 << 16).into(),
            "€😀".into(),
            7.into()
        ])
    );
}
