//! djs-embed's integration tests (one binary, D2791): each requirement of the embedding
//! API (0240 E1-E7) through the public API only.

mod bench;
mod calls;
mod data;
mod example;
mod failures;
mod image;
mod modules;

use djs_embed::{Data, Error, Image, Instance, Limits, Source};

/// An image of one script, without host functions.
pub fn image(src: &str) -> Image {
    Image::builder()
        .source(Source::script("t.js", src))
        .build()
        .unwrap_or_else(|e| panic!("{e}"))
}

pub fn instance(src: &str) -> Instance {
    image(src).instantiate(Limits::default()).unwrap()
}

/// Calls `path` without host functions.
pub fn call(i: &mut Instance, path: &str) -> Result<Data, Error> {
    i.call(path, &[], |c| panic!("unexpected host call {}", c.fn_id()))
}
