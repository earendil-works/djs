//! The cross-build check's program (scripts/cross-check.sh): built for each target a Rust
//! host ships for, it runs a script with a sync and an async host function through an async
//! entry, prints the result and a few numbers whose bits depend on the math and number
//! formatting code, and compares the base image this binary embeds (made by djs-prelude's
//! build script on the build machine) with the one this target makes itself. Every target
//! must print the same lines.

use djs_embed::{Data, Error, HostFn, Limits, Snapshot, Source, Step, Value};

const LOG: u32 = 1;
const FETCH: u32 = 2;

const POLICY: &str = r#"
async function onRequest(req) {
    log("request " + req.url);
    const body = await fetchText(req.url);
    return {
        allow: !body.includes("secret"),
        length: body.length,
        math: [Math.sin(1e22), Math.pow(1.1, 300), Math.exp(-7.5), Math.cbrt(3),
               Math.atan2(1, 3), Math.hypot(3e200, 4e200), Math.log1p(1e-10),
               Math.sqrt(2), Math.fround(0.1)].map(String).join(" "),
        text: [(0.1).toFixed(20), (123.456).toExponential(10), String(2 ** -1074),
               parseFloat("1.7976931348623157e308").toString(36).slice(0, 20)].join(" "),
        sorted: [3, 1, 2, 10].sort().join(","),
        regexp: "a1b22c333".replace(/\d+/g, (m) => m.length),
        utf8: new TextEncoder().encode(body).join(",") + " " +
              new TextDecoder().decode(new Uint8Array([0xe2, 0x82, 0xac, 0xf0, 0x9f, 0xff])),
    };
}
"#;

/// FNV-1a, 64 bits: enough to tell two images apart in the output.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn main() -> Result<(), Error> {
    let snapshot = Snapshot::builder()
        .host_fn(HostFn::sync(LOG, "log"))
        .host_fn(HostFn::asynchronous(FETCH, "fetchText"))
        .source(Source::script("policy.js", POLICY))
        .build()?;
    let mut vm = snapshot.instantiate(Limits::default())?;
    let req = Data::object([("url", Data::from("https://example.com/"))]);
    vm.start("onRequest", &[Value::Data(&req)])?;
    let result = loop {
        match vm.step()? {
            Step::Done => break vm.result()?.to_data()?,
            Step::Yield => {}
            Step::HostCall(id) => {
                let call = vm.host_call(id)?;
                println!("log: {}", call.arg(0).as_str().unwrap_or_default());
                call.reply(Ok(Value::Undefined))?;
            }
            Step::Wait(started) => {
                for id in started {
                    let url = vm.host_call(id)?.arg(0).as_str().map(|s| s.into_owned());
                    let body = format!("body of {} \u{e9}", url.unwrap_or_default());
                    vm.resolve(id, Ok(Value::Str(&body)))?;
                }
            }
        }
    };
    for key in [
        "allow", "length", "math", "text", "sorted", "regexp", "utf8",
    ] {
        println!("{key}: {:?}", result.get(key));
    }
    println!("fuel used: {}", vm.fuel_used());
    println!("snapshot image: {} bytes", snapshot.image_len());

    // The embedded base and the one this target makes: the same bytes, or a cross build
    // embeds an image its target would not have made.
    let embedded = djs_prelude::BASE;
    let mut own = djs_core::vm::Vm::new(djs_core::vm::VmOptions::default())
        .map_err(|e| Error::Usage(format!("cannot make the base: {e:?}")))?;
    own.prepare_base_image()
        .map_err(|e| Error::Usage(format!("cannot make the base image: {e:?}")))?;
    println!(
        "embedded base: {} bytes, fnv {:016x}",
        embedded.len(),
        fnv(embedded)
    );
    let same = own.bytes() == embedded;
    println!(
        "base made here: {}",
        if same { "identical" } else { "DIFFERENT" }
    );
    if !same {
        println!(
            "  {} bytes, fnv {:016x}",
            own.bytes().len(),
            fnv(own.bytes())
        );
        std::process::exit(1);
    }
    Ok(())
}
