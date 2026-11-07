//! The crate docs' example (src/lib.rs), run as written there.

use djs_embed::{Data, Error, HostFn, Limits, Snapshot, Source, Step, Throw, Value};

const LOG: u32 = 1;
const FETCH: u32 = 2;

#[test]
fn the_crate_example_runs() -> Result<(), Error> {
    let snapshot = Snapshot::builder()
        .host_fn(HostFn::sync(LOG, "log"))
        .host_fn(HostFn::asynchronous(FETCH, "fetchText"))
        .global("config", Data::object([("greeting", Data::from("hello"))]))
        .source(Source::script(
            "policy.js",
            r#"
            async function onRequest(req) {
                log(config.greeting + " " + req.url);
                const body = await fetchText(req.url);
                return { allow: !body.includes("secret"), length: body.length };
            }
        "#,
        ))
        .build()?;

    let mut vm = snapshot.instantiate(Limits::default())?;
    let req = Data::object([("url", Data::from("https://example.com/"))]);
    vm.start("onRequest", &[Value::Data(&req)])?;
    let decision = loop {
        match vm.step()? {
            Step::Done => break vm.result()?.to_data()?,
            Step::Yield => {} // a slice of fuel is used up: yield the thread if it is shared
            Step::HostCall(id) => {
                // A sync host function: answer before the script goes on.
                let call = vm.host_call(id)?;
                println!("log: {}", call.arg(0).as_str().unwrap_or_default());
                call.reply(Ok(Value::Undefined))?;
            }
            Step::Wait(started) => {
                // Async host functions: start the work, answer when it is done.
                for id in started {
                    let url = vm.host_call(id)?.arg(0).as_str().map(|s| s.into_owned());
                    match url {
                        Some(url) => vm.resolve(id, Ok(Value::Str(&format!("body of {url}"))))?,
                        None => vm.resolve(id, Err(Throw::type_error("fetchText: no URL")))?,
                    }
                }
            }
        }
    };
    assert_eq!(decision.get("allow"), Some(&Data::Bool(true)));
    vm.reset()?; // ready for the next call, in the snapshot's state
    assert_eq!(decision.get("length"), Some(&Data::from(28)));
    Ok(())
}
