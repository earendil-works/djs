//! djs-embed's integration tests (one binary, D2791): each requirement of the embedding
//! API (0240 E1-E7) through the public API only.

mod bench;
mod calls;
mod data;
mod example;
mod failures;
mod snapshot;

use djs_embed::{Data, Error, Instance, Limits, Snapshot, Source};

/// A snapshot of one script, without host functions.
pub fn snapshot(src: &str) -> Snapshot {
    Snapshot::builder()
        .source(Source::script("t.js", src))
        .build()
        .unwrap_or_else(|e| panic!("{e}"))
}

pub fn instance(src: &str) -> Instance {
    snapshot(src).instantiate(Limits::default()).unwrap()
}

/// Calls `path` without host functions.
pub fn call(i: &mut Instance, path: &str) -> Result<Data, Error> {
    i.call(path, &[], |c| panic!("unexpected host call {}", c.fn_id()))
}
