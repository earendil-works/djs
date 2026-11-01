//! The single-pass compiler: source text to function templates (docs/compiler.md).
//!
//! P0 scope (issue 0014): expressions and statements without functions. Constructs that
//! come later (functions, classes, destructuring, `try`, `for-in`/`for-of`, regex
//! literals, ...) are rejected with a "... not supported yet" SyntaxError.
//!
//! ```ignore
//! let template = compile(&mut heap, src, &CompileOptions::script("main.js"))?;
//! ```
//!
//! The result is a `FuncTemplate` (no captures). The embedder instantiates it as a
//! closure and calls it with `this` = the global object (D008) and, in async-body mode,
//! the parameters.

mod expr;
mod func;
mod parser;
mod resolve;
mod scan;
mod stmt;

#[cfg(test)]
mod tests;

use alloc::borrow::Cow;
use alloc::string::String;
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
}

impl<'a> CompileOptions<'a> {
    pub fn script(filename: &'a str) -> Self {
        CompileOptions {
            goal: Goal::Script,
            filename,
            params: &[],
        }
    }

    pub fn async_body(filename: &'a str, params: &'a [&'a str]) -> Self {
        CompileOptions {
            goal: Goal::AsyncBody,
            filename,
            params,
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
    result.map_err(|e| {
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
    })
}
