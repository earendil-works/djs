//! The single-pass compiler: source text to function templates (docs/compiler.md).
//!
//! Scope so far: expressions and statements (0014), functions, arrows and closures
//! (0024), objects (0027), exceptions (0025), for-in/of (0029), destructuring and
//! tagged templates (0036), async functions, generators and `for await` (0041), classes
//! (0040). Constructs that come later (regex literals, ...) are rejected with a
//! "... not supported yet" SyntaxError.
//!
//! ```ignore
//! let template = compile(&mut heap, src, &CompileOptions::script("main.js"))?;
//! ```
//!
//! The result is a `FuncTemplate` (no captures). The embedder instantiates it as a
//! closure and calls it with `this` = the global object (D008) and, in async-body mode,
//! the parameters.

mod class;
mod expr;
mod for_in_of;
mod func;
mod function;
mod generator;
mod module;
mod parser;
mod pattern;
mod resolve;
mod scan;
mod stmt;
mod ts;
mod ts_decl;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod ts_tests;

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::heap::{Heap, HeapError, Ref};
use crate::lexer::{LexError, LineTable};

/// What the source is compiled as (D004, D008).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Goal {
    /// A Script: top-level `var` creates global object properties, top-level
    /// `let`/`const` go into the realm's global lexical record, `return` is an error.
    Script,
    /// A codemode body: pi runs `(async (tools, console) => { <code> })`, so top-level
    /// declarations are locals, the `params` are parameters, `return` ends the run.
    AsyncBody,
}

#[derive(Clone, Copy, Debug)]
pub struct CompileOptions<'a> {
    pub goal: Goal,
    /// For stack traces and error messages.
    pub filename: &'a str,
    /// Parameter names in [`Goal::AsyncBody`] mode (ignored for scripts).
    pub params: &'a [&'a str],
    /// Privileged mode (docs/compiler.md 7.3): enables `%Name(args)` intrinsic calls
    /// (`bytecode::intrinsics`). Only the embedder sets it, for the prelude; it is never
    /// reachable from user code. Privileged scripts also keep no source text (D1340).
    pub privileged: bool,
    /// TypeScript: the type skipper erases types while parsing, enums, namespaces and
    /// parameter properties compile to code (compiler/ts.rs).
    pub typescript: bool,
}

impl<'a> CompileOptions<'a> {
    pub fn script(filename: &'a str) -> Self {
        CompileOptions {
            goal: Goal::Script,
            filename,
            params: &[],
            privileged: false,
            typescript: false,
        }
    }

    pub fn async_body(filename: &'a str, params: &'a [&'a str]) -> Self {
        CompileOptions {
            goal: Goal::AsyncBody,
            filename,
            params,
            privileged: false,
            typescript: false,
        }
    }

    /// The same options with the TypeScript grammar.
    pub fn ts(self) -> Self {
        CompileOptions {
            typescript: true,
            ..self
        }
    }

    /// A privileged Script (the prelude, D1172).
    pub fn privileged_script(filename: &'a str) -> Self {
        CompileOptions {
            privileged: true,
            ..Self::script(filename)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CompileErrorKind {
    /// A JS SyntaxError.
    Syntax,
    /// The heap ran out of memory while compiling.
    OutOfMemory,
}

/// A compile error with its position: a byte offset and 1-based line/column (columns
/// count code points, D1032).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CompileError {
    pub kind: CompileErrorKind,
    /// QuickJS wording where QuickJS has one (docs/compiler.md 8.1).
    pub message: String,
    pub pos: u32,
    pub line: u32,
    pub col: u32,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            CompileErrorKind::Syntax => write!(f, "SyntaxError: {}", self.message)?,
            CompileErrorKind::OutOfMemory => f.write_str("out of memory")?,
        }
        write!(f, " at {}:{}", self.line, self.col)
    }
}

/// The parser's internal error, positioned in the source.
#[derive(Clone, Debug)]
pub(super) enum PErr {
    Syntax { pos: u32, msg: Cow<'static, str> },
    Oom,
}

impl From<LexError> for PErr {
    fn from(e: LexError) -> Self {
        PErr::Syntax {
            pos: e.pos,
            msg: Cow::Borrowed(e.message),
        }
    }
}

impl From<HeapError> for PErr {
    fn from(_: HeapError) -> Self {
        PErr::Oom
    }
}

pub(super) type PResult<T> = Result<T, PErr>;

/// Compiles `src` (UTF-8) into a function template.
pub fn compile(
    heap: &mut Heap,
    src: &[u8],
    opts: &CompileOptions<'_>,
) -> Result<Ref, CompileError> {
    let result = parser::Parser::new(heap, src, opts).and_then(|p| p.compile());
    result.map(|(t, _)| t).map_err(|e| error_at(src, e))
}

/// TypeScript erasure for the strip-types oracle (TESTING.md): compiles `src` with the
/// TypeScript grammar and returns it with every erased type range blanked (spaces;
/// line breaks stay, so positions do not move). The heap only holds the compiled
/// template, which the caller drops.
pub fn strip_types(
    heap: &mut Heap,
    src: &[u8],
    opts: &CompileOptions<'_>,
) -> Result<Vec<u8>, CompileError> {
    let opts = CompileOptions {
        typescript: true,
        ..*opts
    };
    let result = parser::Parser::new(heap, src, &opts).and_then(|mut p| {
        p.erased = Some(Vec::new());
        p.strip = true;
        p.compile()
    });
    let (_, ranges) = result.map_err(|e| error_at(src, e))?;
    Ok(ts::blank(src, ranges.unwrap_or_default()))
}

fn error_at(src: &[u8], e: PErr) -> CompileError {
    let (kind, pos, message) = match e {
        PErr::Syntax { pos, msg } => (CompileErrorKind::Syntax, pos, String::from(msg)),
        PErr::Oom => (
            CompileErrorKind::OutOfMemory,
            0,
            String::from("out of memory"),
        ),
    };
    let (line, col) = LineTable::new(src).line_col(src, pos);
    CompileError {
        kind,
        message,
        pos,
        line,
        col,
    }
}
