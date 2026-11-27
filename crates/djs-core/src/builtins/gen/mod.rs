//! Generated Rust of the `@native` builtins (0063, ARCH "Generated state machines"):
//! `natives.rs` is written by djs-bgen (`scripts/bgen.sh`) from `builtins/*.ts`; do not
//! edit it. The runtime it calls is `interp/run/native.rs`. `leaves.rs` and `records.rs`
//! are generated likewise.

// Generated code is neither formatted nor linted: it is checked by `bgen --check` and
// by the tests of both backends.
#[rustfmt::skip]
#[allow(
    non_snake_case,
    unused_parens,
    unused_braces,
    unreachable_code,
    unused_labels,
    unused_mut,
    unused_variables,
    clippy::all
)]
pub(crate) mod natives;

// The leaf backend (D3260): `@rust` builtins as straight-line Rust.
#[rustfmt::skip]
#[allow(
    non_snake_case,
    unused_parens,
    unused_braces,
    unreachable_code,
    unused_labels,
    unused_mut,
    unused_variables,
    unused_imports,
    unused_assignments,
    clippy::all
)]
pub(crate) mod leaves;

// The record schemas and named constants (D3710): the `enum`s of builtins/*.ts.
#[rustfmt::skip]
#[allow(clippy::all)]
pub(crate) mod records;
