//! djs-test262's integration tests, one binary: each test binary is linked against the
//! whole engine, and macOS checks every new binary before its first run, so four binaries
//! cost the gate four links and four checks (0183).

mod djs_engine;
mod engine_trait;
mod frontmatter_all;
mod qjs_fixture;
