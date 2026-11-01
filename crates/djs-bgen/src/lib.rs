//! djs-bgen: the build-time compiler for the builtin language (ARCH.md "Builtins").
//! v0: subset checker, resume-point analysis and the JS (bytecode backend) emitter.

pub mod ast;
pub mod diag;
pub mod lexer;
pub mod parser;
