//! The hook benchmark: the per-call cost of a network hook policy run through djs-embed,
//! against the same policy as a native Rust closure.
//!
//! usage: `cargo run --release -p djs-embed --example hooks-bench [-- <calls>]`
//!
//! A port of Gondolin's prototype (native/experiments/djs-hooks, `src/bin/bench.rs`) without
//! its crates: the request head is plain data in ([`Data`]), the body stays host bytes behind
//! an async host function until the policy reads it, and the decision is plain data out
//! (no JSON). The shim (`shim.js`) gives policies a subset of the fetch types over three
//! host functions. URL parsing is the host's: a small parser of absolute URLs here (Gondolin
//! uses its own Rust parser), standing in for the `url` crate of the prototype.
//!
//! Every row includes the harness: cloning the head, and for body rows copying the 64 KiB
//! body into a new buffer (the prototype built a body stream over a copy).

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use djs_embed::{Data, Error, HostFn, Instance, Limits, Snapshot, Source, Step, Throw, Value};

const SHIM: &str = include_str!("shim.js");

const URL_PARSE: u32 = 1;
const BODY_READ: u32 = 2;
const LOG: u32 = 3;

fn policy(name: &str) -> &'static str {
    match name {
        "allowlist" => include_str!("policies/allowlist.js"),
        "body" => include_str!("policies/body.js"),
        "body-bytes" => include_str!("policies/body-bytes.js"),
        "body-text" => include_str!("policies/body-text.js"),
        "noop" => include_str!("policies/noop.js"),
        _ => panic!("no policy {name}"),
    }
}

/// An HTTP request head as the host has it.
#[derive(Clone, Debug, PartialEq)]
struct RequestHead {
    method: String,
    url: String,
    /// Lowercase names, in order
    headers: Vec<(String, String)>,
}

impl RequestHead {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    fn set_header(&mut self, name: &str, value: &str) {
        self.headers.retain(|(k, _)| k != name);
        self.headers.push((name.into(), value.into()));
    }

    fn remove_header(&mut self, name: &str) {
        self.headers.retain(|(k, _)| k != name);
    }

    /// The head as plain data: `{ method, url, headers: [[name, value], ...] }`.
    fn to_data(&self) -> Data {
        let headers = self
            .headers
            .iter()
            .map(|(k, v)| Data::Array(vec![k.as_str().into(), v.as_str().into()]))
            .collect();
        Data::object([
            ("method", self.method.as_str().into()),
            ("url", self.url.as_str().into()),
            ("headers", Data::Array(headers)),
        ])
    }
}

#[derive(Debug, PartialEq)]
struct ResponseHead {
    status: u16,
    status_text: String,
    headers: Vec<(String, String)>,
}

#[derive(Debug, PartialEq)]
enum BodyChoice {
    None,
    Original,
    Body(Vec<u8>),
}

/// What a hook decided about a request.
#[derive(Debug, PartialEq)]
enum Decision {
    Continue,
    Request {
        head: RequestHead,
        body: BodyChoice,
    },
    Respond {
        head: ResponseHead,
        body: BodyChoice,
    },
    Block {
        status: u16,
        status_text: String,
        message: String,
    },
}

trait Hook: Sync {
    fn on_request(&self, head: RequestHead, body: Option<&[u8]>) -> Result<Decision, String>;
}

/// The components of an absolute URL (`scheme://[user[:pass]@]host[:port][/path][?q][#f]`),
/// as the shim's `URL` reads them: `href`, `origin`, `protocol`, ... A stand-in for a real
/// URL parser: lowercases scheme and host, drops default ports, no percent-encoding.
fn parse_url(input: &str) -> Option<[(&'static str, String); 11]> {
    let (scheme, rest) = input.split_once("://")?;
    if scheme.is_empty()
        || !scheme
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"+-.".contains(&b))
    {
        return None;
    }
    let scheme = scheme.to_ascii_lowercase();
    let (rest, hash) = match rest.split_once('#') {
        Some((r, h)) => (r, format!("#{h}")),
        None => (rest, String::new()),
    };
    let (rest, search) = match rest.split_once('?') {
        Some((r, q)) => (r, format!("?{q}")),
        None => (rest, String::new()),
    };
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], rest[i..].to_owned()),
        None => (rest, "/".to_owned()),
    };
    let (userinfo, hostport) = match authority.rsplit_once('@') {
        Some((u, h)) => (u, h),
        None => ("", authority),
    };
    let (username, password) = userinfo.split_once(':').unwrap_or((userinfo, ""));
    let (hostname, port) = match hostport.rsplit_once(':') {
        Some((h, p)) if !h.ends_with(']') || hostport.starts_with('[') => (h, p),
        _ => (hostport, ""),
    };
    if hostname.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let hostname = hostname.to_ascii_lowercase();
    let default = matches!((scheme.as_str(), port), ("http", "80") | ("https", "443"));
    let port = if default { "" } else { port };
    let host = if port.is_empty() {
        hostname.clone()
    } else {
        format!("{hostname}:{port}")
    };
    let origin = format!("{scheme}://{host}");
    let auth = match (username.is_empty(), password.is_empty()) {
        (true, true) => String::new(),
        (false, true) => format!("{username}@"),
        _ => format!("{username}:{password}@"),
    };
    let href = format!("{scheme}://{auth}{host}{path}{search}{hash}");
    Some([
        ("href", href),
        ("origin", origin),
        ("protocol", format!("{scheme}:")),
        ("username", username.to_owned()),
        ("password", password.to_owned()),
        ("host", host),
        ("hostname", hostname),
        ("port", port.to_owned()),
        ("pathname", path),
        ("search", search),
        ("hash", hash),
    ])
}

/// How a hook call gets an instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// A fresh instance from the snapshot per call
    Fresh,
    /// Pooled instances, reset to the snapshot after each call
    Reset,
    /// Pooled long-lived instances (globals persist across calls)
    Pooled,
}

/// A policy compiled with the shim, run through djs-embed.
struct DjsHooks {
    snapshot: Snapshot,
    mode: Mode,
    limits: Limits,
    pool: Mutex<Vec<Instance>>,
    /// Fuel of the last finished call
    last_fuel: AtomicU64,
    /// The head goes in as a [`Data`] tree (owned strings) instead of borrowed [`Value`]s
    data_head: bool,
}

impl DjsHooks {
    fn new(policy: &str, mode: Mode, limits: Limits) -> Result<DjsHooks, Error> {
        let snapshot = Snapshot::builder()
            .host_fn(HostFn::sync(URL_PARSE, "__url_parse"))
            .host_fn(HostFn::asynchronous(BODY_READ, "__body_read"))
            .host_fn(HostFn::sync(LOG, "__log"))
            .source(Source::script("<gondolin-hooks>", SHIM))
            .source(Source::script("policy.js", policy))
            .memory(limits.memory)
            .build()?;
        Ok(DjsHooks {
            snapshot,
            mode,
            limits,
            pool: Mutex::new(Vec::new()),
            last_fuel: AtomicU64::new(0),
            data_head: false,
        })
    }

    fn take(&self) -> Result<Instance, Error> {
        if self.mode != Mode::Fresh
            && let Some(i) = self.pool.lock().unwrap().pop()
        {
            return Ok(i);
        }
        self.snapshot.instantiate(self.limits)
    }

    fn put(&self, mut i: Instance, ok: bool) {
        let keep = match self.mode {
            Mode::Fresh => false,
            Mode::Reset => i.reset().is_ok(),
            // A failed call may have left half-done state in the globals.
            Mode::Pooled => ok,
        };
        if keep {
            self.pool.lock().unwrap().push(i);
        }
    }

    fn run(
        &self,
        i: &mut Instance,
        head: &RequestHead,
        body: Option<&[u8]>,
    ) -> Result<Decision, Error> {
        let stream = Value::Number(if body.is_some() { 1.0 } else { -1.0 });
        if self.data_head {
            i.start(
                "__gondolin_on_request",
                &[Value::Data(&head.to_data()), stream],
            )?;
        } else {
            // The head from borrowed parts: no host allocation per string.
            let pairs: Vec<[Value; 2]> = head
                .headers
                .iter()
                .map(|(k, v)| [Value::Str(k), Value::Str(v)])
                .collect();
            let headers: Vec<Value> = pairs.iter().map(|p| Value::Array(p)).collect();
            let fields = [
                ("method", Value::Str(&head.method)),
                ("url", Value::Str(&head.url)),
                ("headers", Value::Array(&headers)),
            ];
            i.start("__gondolin_on_request", &[Value::Object(&fields), stream])?;
        }
        loop {
            match i.step()? {
                Step::Done => {
                    self.last_fuel.store(i.fuel_used(), Ordering::Relaxed);
                    return decision(i.result()?.to_data()?).map_err(Error::Usage);
                }
                Step::Yield => {}
                Step::HostCall(id) => serve_sync(i, id)?,
                Step::Wait(ids) => {
                    if ids.is_empty() {
                        return Err(Error::Usage("waiting on unanswered host calls".into()));
                    }
                    for id in ids {
                        serve_async(i, id, body)?;
                    }
                }
            }
        }
    }
}

impl Hook for DjsHooks {
    fn on_request(&self, head: RequestHead, body: Option<&[u8]>) -> Result<Decision, String> {
        let mut i = self.take().map_err(|e| e.to_string())?;
        let r = self.run(&mut i, &head, body);
        self.put(i, r.is_ok());
        r.map_err(|e| e.to_string())
    }
}

fn serve_sync(i: &mut Instance, id: djs_embed::CallId) -> Result<(), Error> {
    let call = i.host_call(id)?;
    match call.fn_id() {
        URL_PARSE => {
            let input = call.arg(0).as_str().unwrap_or_default().into_owned();
            let base = call.arg(1).as_str().map(|s| s.into_owned());
            let joined = match base {
                Some(b) if !input.contains("://") => match parse_url(&b) {
                    Some(c) if input.starts_with('/') => Some(format!("{}{input}", c[1].1)),
                    Some(c) => Some(format!(
                        "{}{input}",
                        &c[0].1[..c[0].1.rfind('/').unwrap_or(0) + 1]
                    )),
                    None => None,
                },
                _ => Some(input.clone()),
            };
            match joined.as_deref().and_then(parse_url) {
                Some(c) => {
                    let d = Data::object(c.into_iter().map(|(k, v)| (k, Data::String(v))));
                    call.reply(Ok(Value::Data(&d)))
                }
                None => call.reply(Err(Throw::type_error(format!("Invalid URL: {input}")))),
            }
        }
        LOG => {
            eprintln!("[hook] {}", call.arg(0).as_str().unwrap_or_default());
            call.reply(Ok(Value::Undefined))
        }
        f => call.reply(Err(Throw::type_error(format!("unknown host function {f}")))),
    }
}

fn serve_async(i: &mut Instance, id: djs_embed::CallId, body: Option<&[u8]>) -> Result<(), Error> {
    let call = i.host_call(id)?;
    if call.fn_id() != BODY_READ {
        return call.reply(Err(Throw::type_error("unknown host function")));
    }
    let text = call.arg(1).as_str().as_deref() == Some("text");
    let Some(body) = body else {
        return call.reply(Err(Throw::type_error("no body")));
    };
    if !text {
        return call.reply(Ok(Value::Bytes(body)));
    }
    // The VM decodes the bytes (lossy, as `Response.text()`): one pass over ASCII.
    call.reply(Ok(Value::Utf8(body)))
}

/// Takes the member `key` out of an object's entries.
fn take(entries: &mut Vec<(String, Data)>, key: &str) -> Option<Data> {
    let i = entries.iter().position(|(k, _)| k == key)?;
    Some(entries.swap_remove(i).1)
}

fn string(d: Option<Data>) -> Option<String> {
    match d? {
        Data::String(s) => Some(s),
        _ => None,
    }
}

fn number(d: Option<Data>, default: f64) -> f64 {
    d.and_then(|d| d.as_f64()).unwrap_or(default)
}

/// `[[name, value], ...]` (other members are skipped).
fn pairs(d: Option<Data>) -> Vec<(String, String)> {
    let Some(Data::Array(items)) = d else {
        return Vec::new();
    };
    items
        .into_iter()
        .filter_map(|p| match p {
            Data::Array(p) => match <[Data; 2]>::try_from(p) {
                Ok([Data::String(k), Data::String(v)]) => Some((k, v)),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

fn body_choice(d: Option<Data>) -> BodyChoice {
    let Some(Data::Object(mut d)) = d else {
        return BodyChoice::None;
    };
    match string(take(&mut d, "kind")).as_deref() {
        Some("original") => BodyChoice::Original,
        Some("text") => BodyChoice::Body(string(take(&mut d, "text")).unwrap_or_default().into()),
        Some("bytes") => match take(&mut d, "bytes") {
            Some(Data::Bytes(b)) => BodyChoice::Body(b),
            _ => BodyChoice::Body(Vec::new()),
        },
        _ => BodyChoice::None,
    }
}

/// The shim's decision (plain data) as a [`Decision`], moving its strings out.
fn decision(d: Data) -> Result<Decision, String> {
    let Data::Object(mut d) = d else {
        return Err(format!("bad hook decision: {d:?}"));
    };
    let head = |d: &mut Vec<(String, Data)>| match take(d, "head") {
        Some(Data::Object(h)) => Ok(h),
        _ => Err("bad hook decision: no head".to_owned()),
    };
    match string(take(&mut d, "action")).as_deref() {
        Some("continue") => Ok(Decision::Continue),
        Some("request") => {
            let mut h = head(&mut d)?;
            Ok(Decision::Request {
                head: RequestHead {
                    method: string(take(&mut h, "method")).ok_or("bad hook decision: no method")?,
                    url: string(take(&mut h, "url")).ok_or("bad hook decision: no url")?,
                    headers: pairs(take(&mut h, "headers")),
                },
                body: body_choice(take(&mut d, "body")),
            })
        }
        Some("respond") => {
            let mut h = head(&mut d)?;
            Ok(Decision::Respond {
                head: ResponseHead {
                    status: number(take(&mut h, "status"), 200.0) as u16,
                    status_text: string(take(&mut h, "statusText")).unwrap_or_default(),
                    headers: pairs(take(&mut h, "headers")),
                },
                body: body_choice(take(&mut d, "body")),
            })
        }
        Some("block") => Ok(Decision::Block {
            status: number(take(&mut d, "status"), 403.0) as u16,
            status_text: string(take(&mut d, "statusText")).unwrap_or("Forbidden".into()),
            message: string(take(&mut d, "message")).unwrap_or("request blocked".into()),
        }),
        a => Err(format!("bad hook decision: action {a:?}")),
    }
}

/// Native baseline: the allowlist and body policies as a Rust closure.
struct Native;

impl Hook for Native {
    fn on_request(&self, mut head: RequestHead, body: Option<&[u8]>) -> Result<Decision, String> {
        let respond = |status| Decision::Respond {
            head: ResponseHead {
                status,
                status_text: String::new(),
                headers: vec![],
            },
            body: BodyChoice::None,
        };
        if let Some(body) = body {
            if String::from_utf8_lossy(body).contains("DROP TABLE") {
                return Ok(respond(400));
            }
            return Ok(Decision::Continue);
        }
        let url = parse_url(&head.url).ok_or("Invalid URL")?;
        let host = url[6].1.as_str();
        if !(host == "api.example.com"
            || host.ends_with(".github.com")
            || host == "registry.npmjs.org")
        {
            return Ok(respond(403));
        }
        head.set_header("x-gondolin-policy", "v1");
        head.remove_header("x-debug-token");
        Ok(Decision::Request {
            head,
            body: BodyChoice::Original,
        })
    }
}

fn header_request(host: &str) -> RequestHead {
    let h = |k: &str, v: &str| (k.to_owned(), v.to_owned());
    RequestHead {
        method: "GET".into(),
        url: format!("https://{host}/repos/earendil-works/gondolin/pulls?state=open&per_page=50"),
        headers: vec![
            h("host", host),
            h("user-agent", "curl/8.7.1"),
            h("accept", "application/vnd.github+json"),
            h("accept-encoding", "gzip, deflate, br"),
            h("authorization", "Bearer GONDOLIN_SECRET_GH_TOKEN"),
            h("x-github-api-version", "2022-11-28"),
            h("x-debug-token", "abc"),
            h("connection", "keep-alive"),
        ],
    }
}

fn body_request() -> (RequestHead, Vec<u8>) {
    let mut body = Vec::with_capacity(64 * 1024);
    body.extend_from_slice(b"{\"rows\":[");
    while body.len() < 64 * 1024 - 64 {
        body.extend_from_slice(b"{\"id\":12345,\"name\":\"gondolin\",\"tags\":[\"a\",\"b\"]},");
    }
    body.extend_from_slice(b"{}]}");
    body.resize(64 * 1024, b' ');
    let head = RequestHead {
        method: "POST".into(),
        url: "https://api.example.com/upload".into(),
        headers: vec![
            ("host".into(), "api.example.com".into()),
            ("content-type".into(), "application/json".into()),
            ("content-length".into(), body.len().to_string()),
        ],
    };
    (head, body)
}

fn describe(d: &Decision) -> String {
    match d {
        Decision::Continue => "continue".into(),
        Decision::Request { head, .. } => format!(
            "request x-gondolin-policy={:?} x-debug-token={:?}",
            head.header("x-gondolin-policy"),
            head.header("x-debug-token")
        ),
        Decision::Respond { head, .. } => format!("respond {}", head.status),
        Decision::Block {
            status,
            status_text,
            message,
        } => format!("block {status} {status_text}: {message}"),
    }
}

/// One call with the harness's per-call work: the head cloned, the body copied.
fn one(hooks: &dyn Hook, head: &RequestHead, body: Option<&[u8]>) -> Result<Decision, String> {
    let copy = body.map(<[u8]>::to_vec);
    hooks.on_request(head.clone(), copy.as_deref())
}

struct Stats {
    p50: Duration,
    p99: Duration,
    rate: f64,
}

fn measure(hooks: &dyn Hook, head: &RequestHead, body: Option<&[u8]>, calls: usize) -> Stats {
    for _ in 0..calls.min(200) {
        one(hooks, head, body).unwrap();
    }
    let mut lat = Vec::with_capacity(calls);
    let start = Instant::now();
    for _ in 0..calls {
        let t = Instant::now();
        one(hooks, head, body).unwrap();
        lat.push(t.elapsed());
    }
    let total = start.elapsed();
    lat.sort();
    Stats {
        p50: lat[lat.len() / 2],
        p99: lat[lat.len() * 99 / 100],
        rate: calls as f64 / total.as_secs_f64(),
    }
}

/// Calls/s with `n` callers on threads of their own, sharing `hooks`.
fn concurrent(
    hooks: &dyn Hook,
    head: &RequestHead,
    body: Option<&[u8]>,
    calls: usize,
    n: usize,
) -> f64 {
    let start = Instant::now();
    std::thread::scope(|s| {
        for _ in 0..n {
            s.spawn(|| {
                for _ in 0..calls / n {
                    one(hooks, head, body).unwrap();
                }
            });
        }
    });
    (calls / n * n) as f64 / start.elapsed().as_secs_f64()
}

/// A table row: engine, hook, the backend (and its djs side for fuel), with a body.
type Row<'a> = (&'a str, &'a str, &'a dyn Hook, Option<&'a DjsHooks>, bool);

fn us(d: Duration) -> String {
    format!("{:.1}", d.as_secs_f64() * 1e6)
}

/// `probe <policy> <fresh|reset|pooled> <calls> [data]`: runs one row's calls in a loop, for a
/// sampling profiler or a CPU-time measurement (`/usr/bin/time`), which a loaded machine
/// disturbs less than latencies.
fn probe(args: &[String]) {
    let mode = match args[1].as_str() {
        "fresh" => Mode::Fresh,
        "reset" => Mode::Reset,
        _ => Mode::Pooled,
    };
    let mut hooks = DjsHooks::new(policy(&args[0]), mode, Limits::default()).unwrap();
    hooks.data_head = args.get(3).map(String::as_str) == Some("data");
    let calls: u64 = args[2].parse().unwrap();
    let (bhead, bdata) = body_request();
    let gh = header_request("api.github.com");
    let (head, body) = if args[0].starts_with("body") {
        (&bhead, Some(&bdata[..]))
    } else {
        (&gh, None)
    };
    let start = Instant::now();
    for _ in 0..calls {
        one(&hooks, head, body).unwrap();
    }
    println!(
        "{:.2} µs/call",
        start.elapsed().as_secs_f64() * 1e6 / calls as f64
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("probe") {
        return probe(&args[1..]);
    }
    let calls: usize = args.first().and_then(|a| a.parse().ok()).unwrap_or(5000);
    let limits = Limits::default();
    let make = |name: &str, mode| DjsHooks::new(policy(name), mode, limits).unwrap();

    let t = Instant::now();
    let fresh = make("allowlist", Mode::Fresh);
    println!(
        "compile shim + allowlist policy into a snapshot: {:.1} ms (image {} KiB)",
        t.elapsed().as_secs_f64() * 1e3,
        fresh.snapshot.image_len() / 1024
    );
    let t = Instant::now();
    drop(fresh.snapshot.instantiate(limits).unwrap());
    println!(
        "instantiate (cold): {:.1} µs",
        t.elapsed().as_secs_f64() * 1e6
    );
    let reset = make("allowlist", Mode::Reset);
    let pooled = make("allowlist", Mode::Pooled);
    let mut pooled_data = make("allowlist", Mode::Pooled);
    pooled_data.data_head = true;
    let noop = make("noop", Mode::Pooled);
    let text_only = make("body-text", Mode::Pooled);
    let bytes_only = make("body-bytes", Mode::Pooled);
    let fresh_body = make("body", Mode::Fresh);
    let reset_body = make("body", Mode::Reset);
    let pooled_body = make("body", Mode::Pooled);

    // Sanity: the decisions.
    for host in ["api.github.com", "evil.example.org", "api.example.com"] {
        let d = one(&pooled, &header_request(host), None).unwrap();
        let n = one(&Native, &header_request(host), None).unwrap();
        println!("  {host}: {} (native: {})", describe(&d), describe(&n));
    }
    let (bhead, bdata) = body_request();
    println!(
        "  64 KiB body: {}",
        describe(&one(&pooled_body, &bhead, Some(&bdata)).unwrap())
    );
    let mut bad = bdata.clone();
    bad[100..110].copy_from_slice(b"DROP TABLE");
    println!(
        "  64 KiB body (bad): {}",
        describe(&one(&pooled_body, &bhead, Some(&bad)).unwrap())
    );

    // Fail closed.
    let gh = header_request("api.github.com");
    let fails = |label: &str, src: &str, limits: Limits| {
        let r = DjsHooks::new(src, Mode::Fresh, limits).map_err(|e| e.to_string());
        let r = r.and_then(|h| one(&h, &gh, None));
        println!(
            "  {label}: {}",
            r.map_or_else(|e| e, |d| format!("unexpected: {}", describe(&d)))
        );
    };
    let tight = Limits {
        fuel: 20_000,
        slice: 10_000,
        ..limits
    };
    fails(
        "fuel exhausted",
        "async function onRequest(r) { for (;;) {} }",
        tight,
    );
    fails(
        "memory exhausted",
        "async function onRequest(r) { const a = []; for (;;) a.push(new Array(1000).fill(r.url)); }",
        Limits {
            memory: 4 << 20,
            ..limits
        },
    );
    fails("bad return", "function onRequest(r) { return 42; }", limits);
    fails(
        "never settles",
        "function onRequest(r) { return new Promise(() => {}); }",
        limits,
    );
    fails(
        "blocked",
        "function onRequest(r) { throw new HttpRequestBlockedError('no', 451, 'Unavailable'); }",
        limits,
    );
    fails("syntax error", "function (", limits);

    println!();
    println!(
        "| engine | hook | p50 µs | p99 µs | calls/s (1 caller) | calls/s (8 callers) | fuel/call |"
    );
    println!("|---|---|---:|---:|---:|---:|---:|");
    let rows: [Row; 12] = [
        ("native Rust", "header-only", &Native, None, false),
        ("djs pooled", "empty policy", &noop, Some(&noop), false),
        ("djs fresh", "header-only", &fresh, Some(&fresh), false),
        ("djs reset", "header-only", &reset, Some(&reset), false),
        ("djs pooled", "header-only", &pooled, Some(&pooled), false),
        (
            "djs pooled",
            "header-only, head as a Data tree",
            &pooled_data,
            Some(&pooled_data),
            false,
        ),
        ("native Rust", "64 KiB body", &Native, None, true),
        (
            "djs pooled",
            "64 KiB read as text, unused",
            &text_only,
            Some(&text_only),
            true,
        ),
        (
            "djs pooled",
            "64 KiB read as bytes, unused",
            &bytes_only,
            Some(&bytes_only),
            true,
        ),
        (
            "djs fresh",
            "64 KiB body",
            &fresh_body,
            Some(&fresh_body),
            true,
        ),
        (
            "djs reset",
            "64 KiB body",
            &reset_body,
            Some(&reset_body),
            true,
        ),
        (
            "djs pooled",
            "64 KiB body",
            &pooled_body,
            Some(&pooled_body),
            true,
        ),
    ];
    for (name, kind, hooks, djs, with_body) in rows {
        let (head, body) = if with_body {
            (&bhead, Some(&bdata[..]))
        } else {
            (&gh, None)
        };
        let s = measure(hooks, head, body, calls);
        let c = concurrent(hooks, head, body, calls * 2, 8);
        let fuel = djs.map_or("–".into(), |h| {
            h.last_fuel.load(Ordering::Relaxed).to_string()
        });
        println!(
            "| {name} | {kind} | {} | {} | {:.0} | {:.0} | {fuel} |",
            us(s.p50),
            us(s.p99),
            s.rate,
            c
        );
    }
}
