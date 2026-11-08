//! E2: snapshots, instances and reset.

use djs_embed::{Data, Error, HostFn, Limits, Snapshot, Source};

use crate::{call, instance, snapshot};

#[test]
fn instances_start_in_the_snapshot_state_and_share_nothing() {
    let snap = snapshot("let n = 0; function bump() { return ++n; }");
    let mut a = snap.instantiate(Limits::default()).unwrap();
    let mut b = snap.instantiate(Limits::default()).unwrap();
    assert_eq!(call(&mut a, "bump").unwrap(), 1.into());
    assert_eq!(call(&mut a, "bump").unwrap(), 2.into());
    assert_eq!(call(&mut b, "bump").unwrap(), 1.into());
    a.reset().unwrap();
    assert_eq!(call(&mut a, "bump").unwrap(), 1.into());
}

#[test]
fn snapshots_are_shared_across_threads() {
    let snap = snapshot("function f(x) { return x * 2; }");
    let handles: Vec<_> = (0..4)
        .map(|t| {
            let snap = snap.clone();
            std::thread::spawn(move || {
                let mut i = snap.instantiate(Limits::default()).unwrap();
                i.call("f", &[(t as f64).into()], |_| Ok(())).unwrap()
            })
        })
        .collect();
    let out: Vec<Data> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(out, [0.into(), 2.into(), 4.into(), 6.into()]);
}

#[test]
fn instances_move_between_threads() {
    let mut i = instance("let n = 0; function bump() { return ++n; }");
    call(&mut i, "bump").unwrap();
    let mut i = std::thread::spawn(move || {
        call(&mut i, "bump").unwrap();
        i
    })
    .join()
    .unwrap();
    assert_eq!(call(&mut i, "bump").unwrap(), 3.into());
}

#[test]
fn setup_order_globals_and_frozen_builtins() {
    let snap = Snapshot::builder()
        .host_fn(HostFn::sync(7, "hostLog"))
        .global("config", Data::object([("limit", Data::from(3))]))
        .source(Source::script("a.js", "var fromA = typeof hostLog + ':' + config.limit;"))
        .source(Source::script("b.js", "function get() { return fromA; }\n\
             function patch() { try { Array.prototype.push = null; } catch (e) { return e.name; } }"))
        .build()
        .unwrap();
    let mut i = snap.instantiate(Limits::default()).unwrap();
    assert_eq!(call(&mut i, "get").unwrap(), "function:3".into());
    // Frozen builtins: the assignment is refused.
    assert_eq!(call(&mut i, "patch").unwrap(), "TypeError".into());
    assert_eq!(snap.host_fns(), [HostFn::sync(7, "hostLog")]);
    assert!(snap.image_len() > 10_000);
    let open = Snapshot::builder()
        .frozen_builtins(false)
        .source(Source::script(
            "b.js",
            "function patch() { Array.prototype.x = 1; return [].x; }",
        ))
        .build()
        .unwrap();
    let mut i = open.instantiate(Limits::default()).unwrap();
    assert_eq!(call(&mut i, "patch").unwrap(), 1.into());
}

#[test]
fn setup_failures() {
    let err = |src: &str| {
        Snapshot::builder()
            .host_fn(HostFn::sync(1, "h"))
            .source(Source::script("bad.js", src))
            .build()
            .unwrap_err()
    };
    match err("let = ;") {
        Error::Syntax { source, message } => {
            assert_eq!(source, "bad.js");
            assert!(message.contains("1:"), "{message}");
        }
        e => panic!("{e:?}"),
    }
    match err("throw new TypeError('no')") {
        Error::Exception(e) => assert_eq!(e.message, "TypeError: no"),
        e => panic!("{e:?}"),
    }
    assert!(matches!(err("h()"), Error::Usage(_)));
    let spin = Snapshot::builder()
        .fuel(10_000)
        .source(Source::script("spin.js", "for (;;) {}"))
        .build();
    assert_eq!(spin.unwrap_err(), Error::OutOfFuel { budget: 10_000 });
}

#[test]
fn the_clock_is_the_hosts() {
    let mut i = instance("function now() { return Date.now(); }");
    assert_eq!(call(&mut i, "now").unwrap(), 0.into());
    i.set_now(1.5e12).unwrap();
    assert_eq!(call(&mut i, "now").unwrap(), 1.5e12.into());
}

#[test]
fn reset_keeps_the_grown_heap_until_shrunk() {
    let snap = snapshot(
        "function big(n) { return 'x'.repeat(n).split('').length; }\n\
         function small() { return 1; }",
    );
    let mut i = snap.instantiate(Limits::default()).unwrap();
    let fresh = i.heap_size();
    assert!(i.heap_used() <= fresh);
    assert_eq!(i.heap_used(), snap.image_len());
    let out = i.call("big", &[(1u32 << 20).into()], |_| Ok(())).unwrap();
    assert_eq!(out, Data::from(f64::from(1u32 << 20)));
    i.reset().unwrap();
    let grown = i.heap_size();
    assert!(grown > 4 << 20, "{grown}");
    assert_eq!(i.heap_used(), snap.image_len());
    i.shrink_heap(0).unwrap();
    assert!(i.heap_size() < grown / 4, "{} of {grown}", i.heap_size());
    assert!(i.heap_size() >= i.heap_used());
    // Works as before, and grows again when it needs to.
    assert_eq!(call(&mut i, "small").unwrap(), 1.into());
    i.call("big", &[(1u32 << 20).into()], |_| Ok(())).unwrap();
    assert!(i.heap_size() > 4 << 20);
}
