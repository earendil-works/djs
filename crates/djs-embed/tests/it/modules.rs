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
