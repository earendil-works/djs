//! ES modules (0243): a policy module with `@sdk/...` synthetic imports and helper modules
//! from an in-memory map, compiled once into a snapshot of the evaluated graph; module
//! exports as entry points and read by the host; `import()` answered inside `step`.

use djs_embed::{
    Data, Error, HostFn, Limits, Loader, ModuleMap, ModuleSource, Snapshot, Source, Step,
    SyntheticModule, Value,
};

const ALLOW: u32 = 1;
const LOOKUP: u32 = 2;

const POLICY: &str = r#"
import { allowHosts, lookup, secret } from "@sdk/net";
import { normalize } from "./lib/hosts.js";
import * as sdk from "@sdk/net";

export const loadedAt = globalThis.setupRuns = (globalThis.setupRuns ?? 0) + 1;
export let calls = 0;

export async function onRequest(req) {
    calls++;
    const host = normalize(req.host);
    if (!allowHosts(host)) return { allow: false, host };
    const ip = await lookup(host);
    return { allow: true, host, ip, auth: secret.token, keys: Object.keys(sdk), calls };
}

export async function lazy(name) {
    const m = await import(name);
    return m.describe ? m.describe() : Object.keys(m);
}
"#;

fn files() -> ModuleMap {
    ModuleMap::new()
        .module(
            "lib/hosts.js",
            "import { suffix } from './suffix.js';\n\
             export function normalize(h) { return h.toLowerCase() + suffix; }",
        )
        .module("lib/suffix.js", "export const suffix = '';")
        .module(
            "lib/lazy.js",
            "import { secret } from '@sdk/net';\n\
             export function describe() { return 'lazy ' + secret.token; }",
        )
        .module("lib/throws.js", "throw new TypeError('lazy failed');")
}

fn sdk() -> SyntheticModule {
    SyntheticModule::new("@sdk/net")
        .host_fn(HostFn::sync(ALLOW, "allowHosts"))
        .host_fn(HostFn::asynchronous(LOOKUP, "lookup"))
        .data("secret", Data::object([("token", Data::from("t0k"))]))
}

fn policy_snapshot() -> Snapshot {
    Snapshot::builder()
        .module(sdk())
        .loader(files())
        .source(Source::module("policy.js", POLICY))
        .build()
        .unwrap_or_else(|e| panic!("{e}"))
}

/// Answers the sdk's host functions: example.com is allowed, lookups give a fixed address.
fn host(c: djs_embed::HostCall<'_>) -> Result<(), Error> {
    match c.fn_id() {
        ALLOW => {
            let ok = c.arg(0).as_str().is_some_and(|h| h == "example.com");
            c.reply(Ok(Value::Bool(ok)))
        }
        LOOKUP => c.reply(Ok(Value::Str("192.0.2.1"))),
        id => panic!("unexpected host call {id}"),
    }
}

fn request(host: &str) -> Data {
    Data::object([("host", Data::from(host))])
}

#[test]
fn a_policy_module_with_sdk_imports() {
    let snap = policy_snapshot();
    let mut i = snap.instantiate(Limits::default()).unwrap();
    let req = request("Example.COM");
    let out = i
        .call_export("policy.js", "onRequest", &[Value::Data(&req)], host)
        .unwrap();
    assert_eq!(out.get("allow"), Some(&Data::Bool(true)));
    assert_eq!(out.get("host"), Some(&Data::from("example.com")));
    assert_eq!(out.get("ip"), Some(&Data::from("192.0.2.1")));
    assert_eq!(out.get("auth"), Some(&Data::from("t0k")));
    assert_eq!(
        out.get("keys"),
        Some(&Data::Array(vec![
            Data::from("allowHosts"),
            Data::from("lookup"),
            Data::from("secret"),
        ]))
    );
    let req = request("evil.test");
    let out = i
        .call_export("policy.js", "onRequest", &[Value::Data(&req)], host)
        .unwrap();
    assert_eq!(out.get("allow"), Some(&Data::Bool(false)));
    // The instance kept its module state; the export shows it (M7).
    assert_eq!(
        i.export("policy.js", "calls").unwrap().unwrap().as_number(),
        Some(2.0)
    );
    // The snapshot holds the evaluated graph: setup ran the module once, and a reset or
    // a fresh instance starts from there without running it again.
    i.reset().unwrap();
    assert_eq!(
        i.export("policy.js", "calls").unwrap().unwrap().as_number(),
        Some(0.0)
    );
    let mut fresh = snap.instantiate(Limits::default()).unwrap();
    for vm in [&mut i, &mut fresh] {
        assert_eq!(
            vm.export("policy.js", "loadedAt")
                .unwrap()
                .unwrap()
                .as_number(),
            Some(1.0)
        );
        let req = request("example.com");
        let out = vm
            .call_export("policy.js", "onRequest", &[Value::Data(&req)], host)
            .unwrap();
        assert_eq!(out.get("calls"), Some(&Data::from(1.0)));
    }
    // Imported modules' exports are readable too; unknown ones are None.
    assert_eq!(
        i.export("lib/suffix.js", "suffix")
            .unwrap()
            .unwrap()
            .as_str()
            .as_deref(),
        Some("")
    );
    assert!(i.export("policy.js", "nope").unwrap().is_none());
    assert!(i.export("nope.js", "calls").unwrap().is_none());
}

#[test]
fn stepping_an_exported_entry() {
    let snap = policy_snapshot();
    let mut i = snap.instantiate(Limits::default()).unwrap();
    let req = request("example.com");
    i.start_export("policy.js", "onRequest", &[Value::Data(&req)])
        .unwrap();
    let mut seen = Vec::new();
    let out = loop {
        match i.step().unwrap() {
            Step::Done => break i.result().unwrap().to_data().unwrap(),
            Step::Yield => {}
            Step::HostCall(id) => {
                seen.push("sync");
                i.host_call(id)
                    .unwrap()
                    .reply(Ok(Value::Bool(true)))
                    .unwrap();
            }
            Step::Wait(ids) => {
                for id in ids {
                    seen.push("async");
                    i.resolve(id, Ok(Value::Str("192.0.2.9"))).unwrap();
                }
            }
        }
    };
    assert_eq!(seen, ["sync", "async"]);
    assert_eq!(out.get("ip"), Some(&Data::from("192.0.2.9")));
    let e = i.start_export("policy.js", "missing", &[]).unwrap_err();
    assert_eq!(
        e,
        Error::Usage("module policy.js has no export missing".into())
    );
    let e = i.start_export("other.js", "onRequest", &[]).unwrap_err();
    assert_eq!(e, Error::Usage("no module other.js".into()));
    let e = i.start_export("policy.js", "calls", &[]).unwrap_err();
    assert_eq!(
        e,
        Error::Usage("policy.js export calls is not a function".into())
    );
}

#[test]
fn dynamic_import_in_a_call() {
    let snap = policy_snapshot();
    let mut i = snap.instantiate(Limits::default()).unwrap();
    let lazy = |i: &mut djs_embed::Instance, name: &str| {
        i.call_export("policy.js", "lazy", &[Value::Str(name)], host)
    };
    // Resolved against the policy's key; the loaded module imports the synthetic one.
    assert_eq!(
        lazy(&mut i, "./lib/lazy.js").unwrap(),
        Data::from("lazy t0k")
    );
    assert_eq!(
        lazy(&mut i, "@sdk/net").unwrap(),
        Data::Array(vec![
            Data::from("allowHosts"),
            Data::from("lookup"),
            Data::from("secret"),
        ])
    );
    let err = |r: Result<Data, Error>| match r {
        Err(Error::Exception(e)) => e.message,
        r => panic!("{r:?}"),
    };
    assert_eq!(
        err(lazy(&mut i, "./lib/none.js")),
        "Error: cannot find module './lib/none.js' imported by 'policy.js'"
    );
    assert_eq!(
        err(lazy(&mut i, "./lib/throws.js")),
        "TypeError: lazy failed"
    );
    // The loaded modules stay in the instance until a reset.
    assert_eq!(
        i.export("lib/lazy.js", "describe")
            .unwrap()
            .map(|v| v.is_undefined()),
        Some(false)
    );
    i.reset().unwrap();
    assert!(i.export("lib/lazy.js", "describe").unwrap().is_none());
}

#[test]
fn scripts_import_too() {
    let snap = Snapshot::builder()
        .module(sdk())
        .loader(files())
        .source(Source::script(
            "lib/setup.js",
            "var fromSetup; import('./hosts.js').then(m => { fromSetup = m.normalize('A'); });\n\
             function g() { return fromSetup; }\n\
             async function f(spec) { return Object.keys(await import(spec)); }",
        ))
        .build()
        .unwrap();
    let mut i = snap.instantiate(Limits::default()).unwrap();
    // Setup answered the import before it took the snapshot.
    assert_eq!(super::call(&mut i, "g").unwrap(), Data::from("a"));
    let out = i.call("f", &[Value::Str("./suffix.js")], host).unwrap();
    assert_eq!(out, Data::Array(vec![Data::from("suffix")]));
}

#[test]
fn typescript_modules_and_custom_loaders() {
    /// Every specifier `x` is module `x.ts`, made up.
    struct Gen;
    impl Loader for Gen {
        fn resolve(&self, specifier: &str, _referrer: Option<&str>) -> Result<String, String> {
            Ok(format!("{specifier}.ts"))
        }
        fn load(&self, key: &str) -> Result<ModuleSource, String> {
            Ok(ModuleSource::typescript(format!(
                "export interface T {{ n: number }}\nexport const name: string = '{key}';"
            )))
        }
    }
    let snap = Snapshot::builder()
        .loader(Gen)
        .source(Source::typescript_module(
            "main.ts",
            "import { name, T } from 'dep';\n\
             export function f(t: T): string { return name + t.n; }",
        ))
        .build()
        .unwrap();
    let mut i = snap.instantiate(Limits::default()).unwrap();
    let arg = Data::object([("n", Data::from(1.0))]);
    let out = i
        .call_export("main.ts", "f", &[Value::Data(&arg)], host)
        .unwrap();
    assert_eq!(out, Data::from("dep.ts1"));
}

#[test]
fn setup_errors() {
    let build = |src: &str| {
        Snapshot::builder()
            .module(sdk())
            .loader(files())
            .source(Source::module("bad.js", src))
            .build()
            .unwrap_err()
    };
    assert!(
        matches!(build("export const = 1;"), Error::Syntax { source, .. } if source == "bad.js")
    );
    assert_eq!(
        build("import { nope } from '@sdk/net';"),
        Error::Syntax {
            source: "bad.js".into(),
            message: "SyntaxError: Could not find export 'nope' in module '@sdk/net'".into()
        }
    );
    assert_eq!(
        build("import './lib/none.js';"),
        Error::Load {
            module: "bad.js".into(),
            message: "Error: cannot find module './lib/none.js' imported by 'bad.js'".into()
        }
    );
    match build("import './lib/throws.js';") {
        Error::Exception(e) => assert_eq!(e.message, "TypeError: lazy failed"),
        e => panic!("{e:?}"),
    }
    // Without a loader only synthetic modules resolve.
    let e = Snapshot::builder()
        .source(Source::module("m.js", "import './x.js';"))
        .build()
        .unwrap_err();
    assert!(matches!(e, Error::Load { message, .. } if message.contains("no module loader")));
    let e = Snapshot::builder()
        .module(SyntheticModule::new("s").host_fn(HostFn::sync(u32::MAX, "f")))
        .build()
        .unwrap_err();
    assert!(matches!(e, Error::Usage(m) if m.contains("reserved")));
    let e = Snapshot::builder()
        .module(
            SyntheticModule::new("s")
                .data("a", Data::Null)
                .data("a", Data::Null),
        )
        .build()
        .unwrap_err();
    assert_eq!(
        e,
        Error::Usage("module s: SyntaxError: duplicate export 'a'".into())
    );
}

/// Stack traces of module code name each module by its key, with the line and column in
/// its own source (TypeScript erasure keeps them), and a module body as `<anonymous>`.
#[test]
fn stack_traces_of_module_code() {
    let mut files = ModuleMap::new().module(
        "lib/t.js",
        "// c\n\nexport function f() {\n  throw new TypeError('x');\n}\n",
    );
    files.insert(
        "lib/u.ts",
        ModuleSource::typescript(
            "type T = { a: number };\n\
             export function g(x: T): string { return new Error('u' + x.a).stack!; }\n",
        ),
    );
    let snap = Snapshot::builder()
        .loader(files)
        .source(Source::module(
            "main.js",
            "import { f } from './lib/t.js';\nimport { g } from './lib/u.ts';\n\n\
             export function h() {\n    return f();\n}\n\
             export function k() { return g({ a: 1 }); }\n\
             export const top = new Error('m').stack;\n",
        ))
        .build()
        .unwrap_or_else(|e| panic!("{e}"));
    let mut i = snap.instantiate(Limits::default()).unwrap();
    match i.call_export("main.js", "h", &[], host) {
        Err(Error::Exception(e)) => assert_eq!(
            e.stack.as_deref(),
            Some("    at f (lib/t.js:4:22)\n    at h (main.js:5:5)\n")
        ),
        r => panic!("{r:?}"),
    }
    let s = i.call_export("main.js", "k", &[], host).unwrap();
    assert_eq!(
        s,
        Data::from("    at g (lib/u.ts:2:51)\n    at k (main.js:7:23)\n")
    );
    let top = i.export("main.js", "top").unwrap().unwrap();
    assert_eq!(
        top.as_str().as_deref(),
        Some("    at <anonymous> (main.js:8:29)\n")
    );
    let e = Snapshot::builder()
        .loader(ModuleMap::new().module("x.js", "\n\nthrow new Error('top');"))
        .source(Source::module("m.js", "import './x.js';"))
        .build()
        .unwrap_err();
    match e {
        Error::Exception(e) => {
            assert_eq!(e.stack.as_deref(), Some("    at <anonymous> (x.js:3:16)\n"))
        }
        e => panic!("{e:?}"),
    }
}

#[test]
fn json_modules() {
    let files = ModuleMap::new()
        .json(
            "data/hosts.json",
            r#"{"allow": ["example.com", "api.example.com"]}"#,
        )
        .module("lib/x.js", "export const x = 1;");
    let snap = Snapshot::builder()
        .loader(files.clone())
        .source(Source::module(
            "policy.js",
            "import cfg from './data/hosts.json' with { type: 'json' };\n\
             export function allowed(h) { return cfg.allow.includes(h); }\n\
             export async function later() {\n\
               const m = await import('./data/hosts.json', { with: { type: 'json' } });\n\
               return m.default === cfg;\n\
             }",
        ))
        .build()
        .unwrap();
    let mut i = snap.instantiate(Limits::default()).unwrap();
    let out = i
        .call_export(
            "policy.js",
            "allowed",
            &[Value::Str("api.example.com")],
            host,
        )
        .unwrap();
    assert_eq!(out, Data::Bool(true));
    let out = i.call_export("policy.js", "later", &[], host).unwrap();
    assert_eq!(out, Data::Bool(true));
    // JSON only with the attribute, and the attribute only for JSON.
    for (src, msg) in [
        ("import cfg from './data/hosts.json';", "is JSON"),
        (
            "import x from './lib/x.js' with { type: 'json' };",
            "not a JSON module",
        ),
    ] {
        let e = Snapshot::builder()
            .loader(files.clone())
            .source(Source::module("bad.js", src))
            .build()
            .err()
            .unwrap();
        assert!(e.to_string().contains(msg), "{src}: {e}");
    }
}

#[test]
fn top_level_await() {
    let files = ModuleMap::new()
        .module(
            "lib/config.js",
            "export const hosts = await Promise.resolve(['example.com']);",
        )
        .module("lib/never.js", "await new Promise(() => {});");
    let snap = Snapshot::builder()
        .loader(files.clone())
        .source(Source::module(
            "policy.js",
            "import { hosts } from './lib/config.js';\n\
             const extra = await import('./lib/config.js').then(m => m.hosts.length);\n\
             export function allowed(h) { return hosts.includes(h) && extra === 1; }",
        ))
        .build()
        .unwrap();
    let mut i = snap.instantiate(Limits::default()).unwrap();
    let out = i
        .call_export("policy.js", "allowed", &[Value::Str("example.com")], host)
        .unwrap();
    assert_eq!(out, Data::Bool(true));
    // A top-level await that waits on nothing fails the build.
    let e = Snapshot::builder()
        .loader(files)
        .source(Source::module("bad.js", "import './lib/never.js';"))
        .build()
        .err()
        .unwrap();
    assert!(matches!(e, Error::NeverSettles), "{e}");
}
