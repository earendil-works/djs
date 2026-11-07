# The Rust interface

A Rust program embeds djs natively, without wasm, through the
[`crates/djs-embed`](../../crates/djs-embed) crate: a server that runs user-supplied
JavaScript policies on each request, a tool that evaluates configuration scripts, any
host that wants a small sandboxed interpreter in its own process. The crate has its own
version and an API that exposes nothing of the engine's internals, so the engine can
change underneath it. Its crate documentation
([`crates/djs-embed/src/lib.rs`](../../crates/djs-embed/src/lib.rs)) is the reference;
this page is the overview.

> **In short.** The host compiles its setup (host functions, data, scripts) once into a
> snapshot, an image of a VM after that setup. Each call runs in an instance made from
> the snapshot by copying the image, or in one reset to it. A call is stepped in slices:
> the host answers host calls, starts async work while the script waits, and gets the
> settled value of the function's promise as typed data. Fuel, memory and a deadline
> bound every call, and no script can catch them; every failure is one error type, and a
> bug in the engine poisons only the instance it happened in.

## Snapshots

Setup runs once. The builder defines the host functions the scripts may call and any
plain-data globals, then runs the setup sources in order, each to completion:

```rust
let snapshot = Snapshot::builder()
    .host_fn(HostFn::sync(LOG, "log"))
    .host_fn(HostFn::asynchronous(READ_BODY, "readBody"))
    .global("config", Data::object([("allow", Data::from("example.com"))]))
    .source(Source::script("shim.js", SHIM))
    .source(Source::script("policy.js", policy))
    .build()?;
```

A syntax error or a top-level exception fails the build, so a bad policy is rejected
when it is loaded rather than on its first request. The builtins are frozen by default:
a script cannot patch `Array.prototype` or `JSON` to influence the next call. The
snapshot is the heap image of the VM at the end of setup: immutable, shared by reference
counting, safe to use from any thread. It lives in memory only; building it again from
the sources is how a host gets it back.

An instance starts from it by copying the image: there is no parsing, no compiling and
no setup code to run, so making one costs about as much as copying a few hundred
kilobytes. Resetting an instance copies the image into the heap memory the instance
already has, which also saves the allocation. A host chooses per use:

- **A fresh or reset instance per call.** Nothing of one call is visible to the next,
  and results do not depend on which instance served a call.
- **A long-lived instance.** Globals persist from call to call, so a script can cache
  or count. Calls are cheaper by the reset, but the state is per instance.

## Calls

`start` looks a function up by its global path (`"onRequest"`, or `"hooks.onRequest"`
with `this` the object it was found on) and calls it with typed arguments. `step` then
runs up to one slice of fuel and says why it stopped:

```rust
vm.start("onRequest", &[Value::Wire(&head), Value::Number(stream_id)])?;
let decision = loop {
    match vm.step()? {
        Step::Done => break vm.result()?.to_data()?,
        Step::Yield => tokio::task::yield_now().await,
        Step::HostCall(id) => {
            let call = vm.host_call(id)?;
            let reply = serve_sync(call.fn_id(), call.arg(0).as_str());
            call.reply(reply)?;
        }
        Step::Wait(started) => {
            for id in started { pending.push(start_async(&mut vm, id)?); }
            let (id, body) = next_finished(&mut pending).await;
            match body {
                Ok(bytes) => vm.resolve(id, Ok(Value::Bytes(&bytes)))?,
                Err(e) => vm.resolve(id, Err(Throw::error(e.to_string())))?,
            }
        }
    }
};
```

- **Done.** The call settled. For an `async` function this is when its promise settled,
  however many times the script waited on the host before. The value is kept safe from
  the collector until the next call starts. Async host calls that nothing awaited any
  more are dropped with the rest of the call.
- **A host call.** A sync host function was called, and the script waits for the answer
  at the call: for short, bounded work such as parsing or logging.
- **Wait.** The script waits on async host functions: each call returned a promise and
  the script went on until nothing else could run. The step lists the calls made since
  the last wait; the host starts their work and answers them in any order, at any time.
  Each answer settles its promise as a separate task, in the order the host delivered
  them, so the script's view does not depend on how the host's work interleaved.
- **Yield.** A slice of fuel is used up. The host can give its thread to other work and
  step again; a slow script never holds a worker for long.

For a host that answers everything at once (no I/O), `call` runs this loop with a
closure and returns the result as data.

## Values

Arguments and answers go in as typed values: undefined, null, booleans, numbers,
strings, bytes (which become a `Uint8Array`), plain data (`Data`, an owned tree of
those plus arrays and objects) and wire bytes, djs's binary encoding of the same
values. A string or byte slice is copied into the VM's heap once, and an ASCII string is
copied as it is. The heap is a single block of memory that the VM owns and moves as it
grows, so the VM cannot wrap a host's buffer in place: bytes are always that one copy.

Values come out as references into the heap: a host call's arguments and a call's
result. A string reads as `&str` straight from the heap when it is ASCII and in one
piece, and is converted otherwise; a `Uint8Array` reads as a `&[u8]` slice of the heap.
`to_data` and `to_wire` read plain data without running any JavaScript, so a script
cannot run code during the read: they take primitives, arrays, `Uint8Array`s and plain
objects (with their own enumerable data properties, in order), and refuse everything
else with an error that names where it was found, such as
`cannot encode a function at $.headers[2]`. That covers accessors, proxies, class
instances, `Map`, `Date`, symbols, BigInts and cycles. A result object is therefore
exactly what the script built, and a host never parses JSON text.

## Limits

Each instance has its budgets: a heap cap, fuel per call, the fuel per slice, and an
optional wall-clock deadline per call. None of them can be caught or evaded by a script.
Fuel and the deadline are checked between slices, outside the script. Reaching the heap
cap ends the run on the spot, before any `catch` or `finally` block can run, so a policy
cannot swallow its own exhaustion and carry on with a decision; this is the same limit
as for other hosts, with the catchable form turned off (see
[Limits and resource control](limits.md#memory)). The deadline is also there for the
host to read while it awaits async work, so that slow I/O cannot outlast the call.

## Failures

Every outcome that is not a settled value is an error, and the call is then over: the
VM is idle, its pending host calls are gone, and the instance takes the next call. The
host can treat all of them alike, and fail closed:

| outcome | error |
| --- | --- |
| the function throws, or its promise is rejected | `Exception`, with the message and stack |
| the fuel budget is used up | `OutOfFuel` |
| the deadline passed | `Deadline` |
| the heap cap is reached | `OutOfMemory` |
| nothing can run, nothing is pending, and the promise is still pending | `NeverSettles` |
| a result that is not plain data | `Data` |
| the host misused the API | `Usage` |
| a bug in djs | `Panicked`, then `Poisoned` |

An instance that failed may hold half-updated globals; a host that reuses instances
resets one after a failure. A bug in the engine shows as a panic, which the crate
catches at every entry into the engine. It poisons that instance (drop it and make a new
one) and nothing else: djs keeps no state outside the VM's own heap, so the snapshot and
every other instance are untouched. This relies on Rust's default unwinding panics; a
program built to abort on panic ends instead.

## Threads

A snapshot is shared across threads freely. An instance serves one call at a time and
can move between threads, between calls or between steps of one call, which is what an
async host does when a task resumes on another worker. Parallel calls take one instance
each.

## Adding it to a project

djs is not on crates.io yet, so a project depends on the repository: by path while it
develops against a local checkout, and by a pinned revision otherwise:

```toml
[dependencies]
djs-embed = { git = "https://github.com/earendil-works/djs", rev = "<commit>" }
# or, next to a checkout: djs-embed = { path = "../djs/crates/djs-embed" }

[profile.dev.package.djs-core]
opt-level = 1
```

The profile line is for debug builds: unoptimized, the engine runs several times slower.
The crate's one feature is `unicode`, on by default: Unicode property escapes in regular
expressions and `String.prototype.normalize`, about 60 KB of native code. The engine's
other build options (dumps, lazy compilation, the compile cache) stay off, since an
embedder does not use them. What the build pulls in at run time is four crates of the
repository, the engine, its base image, its math functions and the embedding crate,
and no third-party crate.

The engine is plain Rust with no C code and no target-specific build step, so it
cross-compiles with nothing but a linker for the target, for example `zig cc` for
static linux-musl binaries. The base image, the engine's realm with all builtins that
every snapshot starts from, is made at build time on the build machine and embedded;
its format is the same on every target (little-endian cells, 32-bit offsets, the same
math results), so a cross build embeds exactly the image the target would have made.
[`scripts/cross-check.sh`](../../scripts/cross-check.sh) checks this: it builds a small
program for aarch64 and x86_64 macOS and linux-musl, runs each (x86_64 macOS under
Rosetta, linux in a container), and compares their output, the embedded image and an
image each makes itself.

## Where to look

- [`crates/djs-embed/src/lib.rs`](../../crates/djs-embed/src/lib.rs): the reference,
  with an example that
  [`crates/djs-embed/tests/it/example.rs`](../../crates/djs-embed/tests/it/example.rs)
  runs.
- [`crates/djs-embed/examples/cross_check.rs`](../../crates/djs-embed/examples/cross_check.rs):
  the program the cross-build check runs on each target.
- [`crates/djs-embed/src/instance.rs`](../../crates/djs-embed/src/instance.rs): the call
  protocol and the failure handling.
- [`crates/djs-embed/tests/it/failures.rs`](../../crates/djs-embed/tests/it/failures.rs):
  each failure mode.
- [`crates/djs-core/src/wire/encode.rs`](../../crates/djs-core/src/wire/encode.rs): the
  reading of plain data out of the heap;
  [`crates/djs-core/src/wire/writer.rs`](../../crates/djs-core/src/wire/writer.rs): the
  Rust encoder of the wire format.
