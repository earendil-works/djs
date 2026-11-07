//! E6/E7: budgets a script cannot catch or evade, and the failure model.

use std::time::Duration;

use djs_embed::{Error, HostFn, Limits, Snapshot, Source, Step, Value};

use crate::{call, snapshot};

/// The script tries to swallow everything; the last global shows whether a handler ran.
const SWALLOW: &str = "var ran = 'nothing';\n\
    function spin() { try { for (;;) {} } catch (e) { ran = 'catch'; } finally { ran = 'finally'; } }\n\
    function hog() {\n\
      try { const a = []; for (let i = 0; ; i++) a.push('x'.repeat(1000) + i); }\n\
      catch (e) { ran = 'catch'; return 'swallowed'; } finally { ran = 'finally'; }\n\
    }\n\
    async function hogAsync() {\n\
      await null;\n\
      try { const a = []; for (;;) a.push(new Array(1000)); } catch (e) { return 'swallowed'; }\n\
    }\n\
    function ranWhat() { return ran; }\n\
    function never() { return new Promise(() => {}); }\n\
    async function neverAsync() { await new Promise(() => {}); }\n\
    function bad() { return { f: function () {} }; }";

#[test]
fn fuel_runs_out_in_slices() {
    let snap = snapshot(SWALLOW);
    let limits = Limits {
        fuel: 1_000_000,
        slice: 100_000,
        ..Limits::default()
    };
    let mut i = snap.instantiate(limits).unwrap();
    i.start("spin", &[]).unwrap();
    let mut yields = 0;
    let e = loop {
        match i.step() {
            Ok(Step::Yield) => yields += 1,
            Ok(s) => panic!("{s:?}"),
            Err(e) => break e,
        }
    };
    assert_eq!(e, Error::OutOfFuel { budget: 1_000_000 });
    assert_eq!(yields, 9);
    assert!(i.fuel_used() >= 1_000_000);
    // No handler ran, and the instance serves the next call.
    assert_eq!(call(&mut i, "ranWhat").unwrap(), "nothing".into());
}

#[test]
fn the_deadline_is_checked_between_slices() {
    let snap = snapshot(SWALLOW);
    let limits = Limits {
        fuel: u64::MAX,
        slice: 10_000,
        deadline: Some(Duration::from_millis(20)),
        ..Limits::default()
    };
    let mut i = snap.instantiate(limits).unwrap();
    assert!(i.deadline().is_none());
    let t = std::time::Instant::now();
    assert_eq!(call(&mut i, "spin"), Err(Error::Deadline));
    assert!(t.elapsed() < Duration::from_secs(5));
    assert_eq!(call(&mut i, "ranWhat").unwrap(), "nothing".into());
    // While waiting for async host work too: the host reads the deadline.
    let snap = Snapshot::builder()
        .host_fn(HostFn::asynchronous(1, "slow"))
        .source(Source::script(
            "t.js",
            "async function f() { return await slow(); }",
        ))
        .build()
        .unwrap();
    let mut i = snap.instantiate(limits).unwrap();
    i.start("f", &[]).unwrap();
    let Step::Wait(ids) = i.step().unwrap() else {
        panic!()
    };
    assert!(i.deadline().is_some());
    std::thread::sleep(Duration::from_millis(30));
    i.resolve(ids[0], Ok(Value::Null)).unwrap();
    assert_eq!(i.step(), Err(Error::Deadline));
}

#[test]
fn the_memory_cap_cannot_be_caught() {
    let snap = snapshot(SWALLOW);
    let limits = Limits {
        memory: 4 << 20,
        ..Limits::default()
    };
    let mut i = snap.instantiate(limits).unwrap();
    assert_eq!(call(&mut i, "hog"), Err(Error::OutOfMemory));
    // Neither the catch nor the finally ran.
    assert_eq!(call(&mut i, "ranWhat").unwrap(), "nothing".into());
    assert_eq!(call(&mut i, "hogAsync"), Err(Error::OutOfMemory));
    // Usable afterwards, and a reset is a clean start.
    i.reset().unwrap();
    assert_eq!(call(&mut i, "ranWhat").unwrap(), "nothing".into());
}

#[test]
fn a_promise_that_never_settles() {
    let snap = snapshot(SWALLOW);
    let mut i = snap.instantiate(Limits::default()).unwrap();
    assert_eq!(call(&mut i, "never"), Err(Error::NeverSettles));
    assert_eq!(call(&mut i, "neverAsync"), Err(Error::NeverSettles));
    assert_eq!(call(&mut i, "ranWhat").unwrap(), "nothing".into());
}

#[test]
fn a_bad_return_is_a_data_error() {
    let snap = snapshot(SWALLOW);
    let mut i = snap.instantiate(Limits::default()).unwrap();
    assert_eq!(
        call(&mut i, "bad"),
        Err(Error::Data("cannot encode a function at $.f".into()))
    );
}

#[test]
fn a_small_instance_cannot_load_a_big_snapshot() {
    let snap = snapshot("var big = Array.from({ length: 50000 }, (_, i) => 'item ' + i);");
    assert!(snap.image_len() > 1 << 20);
    let small = Limits {
        memory: 1 << 20,
        ..Limits::default()
    };
    assert_eq!(snap.instantiate(small).err(), Some(Error::OutOfMemory));
}

#[test]
fn every_error_displays() {
    let e = Error::OutOfFuel { budget: 5 };
    assert_eq!(e.to_string(), "out of fuel (budget 5)");
    let boxed: Box<dyn std::error::Error> = Box::new(Error::Deadline);
    assert_eq!(boxed.to_string(), "deadline exceeded");
}
