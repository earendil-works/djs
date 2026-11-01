//! The single-pass compiler: source text to function templates (docs/compiler.md).
//!
//! Scope so far: expressions and statements (0014), functions, arrows and closures
//! (0024), objects (0027), exceptions (0025), for-in/of (0029), destructuring and
//! tagged templates (0036), async functions, generators and `for await` (0041), classes
//! (0040), TypeScript (0048), `using` declarations, completion values, eval code and
//! `import()` (0058). Decorators and `accessor` fields are not there yet (a plain
//! SyntaxError).
//!
//! ```ignore
//! let template = compile(&mut heap, src, &CompileOptions::script("main.js"))?;
//! ```
//!
//! The result is a `FuncTemplate` (no captures). The embedder instantiates it as a
//! closure and calls it with `this` = the global object (D008) and, in async-body mode,
//! the parameters.

mod class;
mod decorators;
mod dynamic;
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
mod using;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod ts_tests;

use alloc::borrow::Cow;
use alloc::boxed::Box;
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
    /// Indirect eval code (strict, PerformEval): a sync body whose top-level
    /// declarations are locals of the call, free names are globals, `this` is what the
    /// embedder calls it with (the global object), and the result is the completion
    /// value. `return`, `new.target` and `super` are errors; `await` is an identifier,
    /// as in a Script.
    Eval,
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

    /// Indirect eval code ([`Goal::Eval`]).
    pub fn eval(filename: &'a str) -> Self {
        CompileOptions {
            goal: Goal::Eval,
            ..Self::script(filename)
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

/// The parser's internal error, positioned in the source. Boxed so that a `PResult<T>`
/// with a small `T` is returned in registers: with the 16-byte error inline, each of the
/// compiler's ~800 `?` copied the result through memory (~27 KB of wasm, D1572).
#[derive(Clone, Debug)]
pub(super) struct PErr(Box<PErrKind>);

#[derive(Clone, Debug)]
pub(super) enum PErrKind {
    Syntax { pos: u32, msg: Cow<'static, str> },
    Oom,
}

impl PErr {
    #[cold]
    #[inline(never)]
    pub(super) fn syntax(pos: u32, msg: Cow<'static, str>) -> PErr {
        PErr(Box::new(PErrKind::Syntax { pos, msg }))
    }

    #[cold]
    #[inline(never)]
    pub(super) fn oom() -> PErr {
        PErr(Box::new(PErrKind::Oom))
    }
}

impl From<LexError> for PErr {
    fn from(e: LexError) -> Self {
        PErr::syntax(e.pos, Cow::Borrowed(e.message))
    }
}

impl From<HeapError> for PErr {
    fn from(_: HeapError) -> Self {
        PErr::oom()
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

/// The kinds of CreateDynamicFunction (20.2.1.1.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DynamicKind {
    Normal,
    Generator,
    Async,
    AsyncGenerator,
}

impl DynamicKind {
    /// The sourceText prefix before ` anonymous(`.
    pub const fn prefix(self) -> &'static str {
        match self {
            DynamicKind::Normal => "function",
            DynamicKind::Generator => "function*",
            DynamicKind::Async => "async function",
            DynamicKind::AsyncGenerator => "async function*",
        }
    }
}

/// Compiles the sourceText of CreateDynamicFunction (`<prefix> anonymous(<P>\n) {\n<body>\n}`)
/// as a Script whose top-level code returns the function (compiler/dynamic.rs, D1590).
/// `params_close` is the offset of the `)` after P: a parameter list that ends anywhere
/// else, or anything after the function, is a SyntaxError.
pub fn compile_dynamic_function(
    heap: &mut Heap,
    src: &[u8],
    opts: &CompileOptions<'_>,
    kind: DynamicKind,
    params_close: u32,
) -> Result<Ref, CompileError> {
    let flavor = match kind {
        DynamicKind::Normal => (false, false),
        DynamicKind::Generator => (false, true),
        DynamicKind::Async => (true, false),
        DynamicKind::AsyncGenerator => (true, true),
    };
    let opts = CompileOptions {
        goal: Goal::Script,
        ..*opts
    };
    parser::Parser::new(heap, src, &opts)
        .and_then(|p| p.compile_dynamic(flavor, params_close))
        .map_err(|e| error_at(src, e))
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
    let (kind, pos, message) = match *e.0 {
        PErrKind::Syntax { pos, msg } => (CompileErrorKind::Syntax, pos, String::from(msg)),
        PErrKind::Oom => (
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
