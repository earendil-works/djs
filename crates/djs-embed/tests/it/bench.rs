//! A micro-benchmark of instantiate, reset and a call (0240), for the numbers in the
//! guide: `cargo test --release -p djs-embed --test it bench -- --ignored --nocapture`.

use std::time::Instant;

use djs_embed::{Data, HostFn, Limits, Snapshot, Source, Value};

/// A shim-sized setup: helper classes the call uses, and a policy over a request head.
const SETUP: &str = r#"
class Headers {
  constructor(pairs) { this.map = new Map(); for (const [k, v] of pairs) this.map.set(k, v); }
  get(k) { return this.map.get(k.toLowerCase()) ?? null; }
  set(k, v) { this.map.set(k.toLowerCase(), v); }
  delete(k) { this.map.delete(k.toLowerCase()); }
  entries() { return [...this.map]; }
}
const ALLOW = ["example.com", "*.example.org"];
function allowed(host) {
  return ALLOW.some((p) => p.startsWith("*.") ? host.endsWith(p.slice(1)) : host === p);
}
function empty() {}
async function onRequest(req) {
  const host = req.url.split("/")[2];
  if (!allowed(host)) return { action: "block", status: 403 };
  const h = new Headers(req.headers);
  for (const [k, v] of h.entries()) if (v.includes("SECRET")) return { action: "block", status: 400 };
  h.set("x-policy", "1");
  h.delete("cookie");
  log(host);
  return { action: "request", headers: h.entries() };
}
"#;

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

/// Median µs of `f` over `n` runs.
fn time(n: usize, mut f: impl FnMut()) -> f64 {
    for _ in 0..n / 10 {
        f();
    }
    median(
        (0..n)
            .map(|_| {
                let t = Instant::now();
                f();
                t.elapsed().as_secs_f64() * 1e6
            })
            .collect(),
    )
}

#[test]
#[ignore]
fn bench() {
    let snap = Snapshot::builder()
        .host_fn(HostFn::sync(1, "log"))
        .source(Source::script("policy.js", SETUP))
        .build()
        .unwrap();
    let head = Data::object([
        ("method", Data::from("GET")),
        ("url", "https://api.example.org/v1/items".into()),
        (
            "headers",
            Data::Array(
                [
                    ("host", "api.example.org"),
                    ("accept", "*/*"),
                    ("cookie", "a=b"),
                    ("user-agent", "bench"),
                ]
                .iter()
                .map(|(k, v)| Data::Array(vec![(*k).into(), (*v).into()]))
                .collect(),
            ),
        ),
    ]);
    let wire = head.to_wire();
    let n = 2000;
    println!("snapshot image: {} KiB", snap.image_len() / 1024);
    let instantiate = time(n, || drop(snap.instantiate(Limits::default()).unwrap()));
    let mut inst = snap.instantiate(Limits::default()).unwrap();
    let reset = time(n, || inst.reset().unwrap());
    let empty = time(n, || {
        inst.call("empty", &[], |_| Ok(())).unwrap();
    });
    let policy = |inst: &mut djs_embed::Instance| {
        inst.call("onRequest", &[Value::Wire(&wire)], |c| {
            c.reply(Ok(Value::Undefined))
        })
        .unwrap()
    };
    let pooled = time(n, || {
        policy(&mut inst);
    });
    let with_reset = time(n, || {
        inst.reset().unwrap();
        policy(&mut inst);
    });
    let fresh = time(n, || {
        let mut i = snap.instantiate(Limits::default()).unwrap();
        policy(&mut i);
    });
    println!(
        "instantiate {instantiate:.1} µs, reset {reset:.1} µs, empty call {empty:.1} µs, \
         header policy: pooled {pooled:.1} µs, reset + call {with_reset:.1} µs, \
         fresh instance + call {fresh:.1} µs ({} fuel)",
        inst.fuel_used()
    );
}
