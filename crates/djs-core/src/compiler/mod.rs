//! The single-pass compiler: source text to function templates (docs/compiler.md).
//!
//! Scope so far: expressions and statements (0014), functions, arrows and closures
//! (0024), objects (0027), exceptions (0025), for-in/of (0029), destructuring and
//! tagged templates (0036), async functions, generators and `for await` (0041), classes
//! (0040), TypeScript (0048), `using` declarations, completion values, eval code and
//! `import()` (0058), decorators and `accessor` fields (0065).
//!
//! ```ignore
//! let template = compile(&mut heap, src, &CompileOptions::script("main.js"))?;
//! ```
//!
//! The result is a `FuncTemplate` (no captures). The embedder instantiates it as a
//! closure and calls it with `this` = the global object (D008) and, in async-body mode,
//! the parameters.

mod class;
mod completion;
mod decorators;
mod dynamic;
mod expr;
mod for_in_of;
mod func;
mod function;
mod generator;
mod module;
pub mod module_meta;
mod parser;
mod pattern;
mod peephole;
mod resolve;
mod scan;
mod stmt;
mod temps;
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
    /// A codemode body, run as `(async (params) => { <code> })`: top-level
    /// declarations are locals, the `params` are parameters, `return` ends the run.
    AsyncBody,
    /// Indirect eval code (strict, PerformEval): a sync body whose top-level
    /// declarations are locals of the call, free names are globals, `this` is what the
    /// embedder calls it with (the global object), and the result is the completion
    /// value. `return`, `new.target` and `super` are errors; `await` is an identifier,
    /// as in a Script.
    Eval,
    /// An ECMAScript module ([`compile_module`]): strict, `import`/`export`, top-level
    /// bindings in the module environment the linker sets up, `this` undefined.
    Module,
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
    /// `debugger;` in an async function awaits a call of the global `debugger` (D3050):
    /// a host that binds an async function there makes the statement a suspension point,
    /// where the run can be snapshotted. Outside async functions `debugger` does nothing.
    pub debugger_awaits: bool,
}

impl<'a> CompileOptions<'a> {
    pub fn script(filename: &'a str) -> Self {
        CompileOptions {
            goal: Goal::Script,
            filename,
            params: &[],
            privileged: false,
            typescript: false,
            debugger_awaits: false,
        }
    }

    /// Indirect eval code ([`Goal::Eval`]).
    pub fn eval(filename: &'a str) -> Self {
        CompileOptions {
            goal: Goal::Eval,
            ..Self::script(filename)
        }
    }

    /// A module ([`Goal::Module`]); `filename` is its key.
    pub fn module(filename: &'a str) -> Self {
        CompileOptions {
            goal: Goal::Module,
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
            debugger_awaits: false,
        }
    }

    /// The options that decide the code, as a Script cell keeps them (`Script::OPTIONS`):
    /// the goal in the low two bits, then TypeScript, privileged, `debugger` awaits.
    pub fn bits(&self) -> u32 {
        let goal = match self.goal {
            Goal::Script => 0,
            Goal::AsyncBody => 1,
            Goal::Eval => 2,
            Goal::Module => 3,
        };
        goal | u32::from(self.typescript) << 2
            | u32::from(self.privileged) << 3
            | u32::from(self.debugger_awaits) << 4
    }

    /// `script` with the options of [`CompileOptions::bits`] `bits` (privileged code
    /// keeps no source: never compiled again).
    #[cfg(feature = "snapshot")]
    pub(crate) fn from_bits(filename: &'a str, params: &'a [&'a str], bits: u32) -> Self {
        CompileOptions {
            goal: match bits & 3 {
                1 => Goal::AsyncBody,
                2 => Goal::Eval,
                3 => Goal::Module,
                _ => Goal::Script,
            },
            filename,
            params,
            privileged: bits & 1 << 3 != 0,
            typescript: bits & 1 << 2 != 0,
            debugger_awaits: bits & 1 << 4 != 0,
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
    /// The host could not provide memory below the cap (D1690).
    HostOutOfMemory,
    /// Not an error of the program: it nests deeper than the small stack allows, and the
    /// host runs the compile again on a larger stack (D3280, `Vm::set_elastic_stack`).
    NeedStack,
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

impl CompileError {
    /// The error as text: `SyntaxError: <message> at <line>:<col>` (the Display form,
    /// built without core::fmt for the shipped module, D2339).
    pub fn text(&self) -> String {
        let mut out = String::from(match self.kind {
            CompileErrorKind::Syntax => "SyntaxError: ",
            CompileErrorKind::OutOfMemory => "out of memory",
            CompileErrorKind::HostOutOfMemory => "out of host memory",
            CompileErrorKind::NeedStack => "needs a larger stack",
        });
        if self.kind == CompileErrorKind::Syntax {
            out.push_str(&self.message);
        }
        crate::text::cat_to!(&mut out, " at ", self.line, ":", self.col);
        out
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text())
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
    HostOom,
    NeedStack,
}

impl PErr {
    #[cold]
    #[inline(never)]
    pub(super) fn need_stack() -> PErr {
        PErr(Box::new(PErrKind::NeedStack))
    }

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
    fn from(e: HeapError) -> Self {
        if e == HeapError::HostOutOfMemory {
            PErr(Box::new(PErrKind::HostOom))
        } else {
            PErr::oom()
        }
    }
}

pub(super) type PResult<T> = Result<T, PErr>;

/// Runs a compile `f` under the elastic stack (D3280). The parser and the regexp parser
/// nest only to the small stack's limits unless `Heap::stack_deep` is set; a compile that
/// went deeper sets `Heap::stack_need`. Then the full limits are armed and the compile
/// runs again: here at once (native builds, whose stack is large), or, with
/// `Heap::stack_elastic`, it fails with `need`'s error and the host runs it again on a
/// larger stack. Either way the heap sees the same attempts, so heap images and fuel do
/// not depend on which. Any other end disarms the full limits. Wraps a whole operation
/// (every parse it makes), once.
pub(crate) fn elastic<T, E>(
    heap: &mut Heap,
    mut f: impl FnMut(&mut Heap) -> Result<T, E>,
    need: impl Fn() -> E,
) -> Result<T, E> {
    loop {
        let r = f(heap);
        match again(heap) {
            Some(true) => {}
            Some(false) => return Err(need()),
            None => return r,
        }
    }
}

/// After an attempt of [`elastic`]: `None` when it stands, else whether to run it again
/// now (`Some(false)`: the host does). Out of line: every compile entry has a copy of
/// [`elastic`].
#[inline(never)]
fn again(heap: &mut Heap) -> Option<bool> {
    let need = core::mem::take(&mut heap.stack_need);
    heap.stack_deep = need;
    need.then_some(!heap.stack_elastic)
}

/// [`attempt`] under the elastic stack ([`elastic`]).
fn parse<T>(
    heap: &mut Heap,
    src: &[u8],
    opts: &CompileOptions<'_>,
    f: impl Fn(parser::Parser<'_, '_>) -> PResult<T>,
) -> PResult<T> {
    if needs_props(heap, src) {
        return Err(PErr::need_stack());
    }
    elastic(heap, |heap| attempt(heap, src, opts, &f), PErr::need_stack)
}

/// Whether a compile of `src` stops before it begins for the Unicode property tables
/// (D3350): the build has none, the host hands them over (with the elastic stack), and
/// `src` may hold a property escape (`\p{` or `\P{`, which a regular expression literal
/// can only spell so). Then `Heap::tables_need` has `unicode::PROPS`, the compile fails like one that
/// needs the larger stack and runs again with the tables: no attempt without them has an
/// effect, so heaps and fuel are those of a build with the tables. A source that only
/// mentions the text loads the tables for nothing, which changes no result.
pub(crate) fn needs_props(heap: &mut Heap, src: &[u8]) -> bool {
    #[cfg(feature = "regexp-unicode-props")]
    if heap.props.is_none() && heap.stack_elastic && has_property_escape(src) {
        return heap.need_tables(crate::unicode::PROPS).is_some();
    }
    let _ = (heap, src);
    false
}

/// Whether `src` has `\p{` or `\P{`.
#[cfg(feature = "regexp-unicode-props")]
fn has_property_escape(src: &[u8]) -> bool {
    let mut at = 0;
    while let Some(i) = crate::swar::find_byte(&src[at..], b'\\') {
        at += i + 1;
        if matches!(src.get(at..at + 2), Some([b'p' | b'P', b'{'])) {
            return true;
        }
    }
    false
}

/// Runs `f` on a parser of `src`.
fn attempt<T>(
    heap: &mut Heap,
    src: &[u8],
    opts: &CompileOptions<'_>,
    f: &impl Fn(parser::Parser<'_, '_>) -> PResult<T>,
) -> PResult<T> {
    parser::Parser::new(heap, src, opts).and_then(f)
}

/// Compiles `src` (UTF-8) into a function template.
pub fn compile(
    heap: &mut Heap,
    src: &[u8],
    opts: &CompileOptions<'_>,
) -> Result<Ref, CompileError> {
    if crate::EMBED && opts.goal == Goal::Module {
        return Err(not_a_module_entry(src));
    }
    let result = parse(heap, src, opts, |p| p.compile());
    result.map(|(t, ..)| t).map_err(|e| error_at(src, e))
}

/// Compiles module source `src` ([`Goal::Module`]): the template of the module body and
/// its module record metadata, a U32Array (`compiler::module_meta` describes the words).
/// Without the `embed` feature (D3175) a SyntaxError.
pub fn compile_module(
    heap: &mut Heap,
    src: &[u8],
    opts: &CompileOptions<'_>,
) -> Result<(Ref, Ref), CompileError> {
    let opts = CompileOptions {
        goal: Goal::Module,
        ..*opts
    };
    if !crate::EMBED {
        return Err(error_at(
            src,
            PErr::syntax(0, Cow::Borrowed("modules are not available in this build")),
        ));
    }
    // A module with top-level await is an async body ([[HasTLA]]); which one is, the first
    // parse finds out (`Parser::module_await`): only those compile twice.
    let both = |heap: &mut Heap| {
        let first = parser::Parser::new(heap, src, &opts)
            .map_err(|e| (e, false))
            .and_then(|p| p.compile_module(false));
        match first {
            Ok(r) => Ok(r),
            Err((_, true)) => attempt(heap, src, &opts, &|p| {
                p.compile_module(true).map_err(|e| e.0)
            }),
            Err((e, false)) => Err(e),
        }
    };
    if needs_props(heap, src) {
        return Err(error_at(src, PErr::need_stack()));
    }
    elastic(heap, both, PErr::need_stack).map_err(|e| error_at(src, e))
}

/// The names of the prelude function table's slots (D2440), in slot order, for
/// privileged source `src`: the prelude functions that others refer to. The registry
/// names them by these (vm/registry.rs). Compiles into `heap`.
pub fn prelude_function_names(
    heap: &mut Heap,
    src: &[u8],
    opts: &CompileOptions<'_>,
) -> Result<Vec<crate::heap::Atom>, CompileError> {
    let result = parse(heap, src, opts, |mut p| {
        p.builtin_names = Some(Vec::new());
        p.compile()
    });
    result
        .map(|(.., names)| names.unwrap_or_default())
        .map_err(|e| error_at(src, e))
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
    parse(heap, src, &opts, |p| {
        p.compile_dynamic(flavor, params_close)
    })
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
    let result = parse(heap, src, &opts, |mut p| {
        p.erased = Some(Vec::new());
        p.strip = true;
        p.compile()
    });
    let (_, ranges, _) = result.map_err(|e| error_at(src, e))?;
    Ok(ts::blank(src, ranges.unwrap_or_default()))
}

/// Module code compiles with [`compile_module`] only: it needs its metadata.
#[cold]
fn not_a_module_entry(src: &[u8]) -> CompileError {
    error_at(
        src,
        PErr::syntax(0, Cow::Borrowed("module code compiles with compile_module")),
    )
}

fn error_at(src: &[u8], e: PErr) -> CompileError {
    let (kind, pos, message) = match *e.0 {
        PErrKind::Syntax { pos, msg } => (CompileErrorKind::Syntax, pos, String::from(msg)),
        PErrKind::Oom => (
            CompileErrorKind::OutOfMemory,
            0,
            String::from("out of memory"),
        ),
        PErrKind::HostOom => (
            CompileErrorKind::HostOutOfMemory,
            0,
            String::from("out of host memory"),
        ),
        PErrKind::NeedStack => (CompileErrorKind::NeedStack, 0, String::new()),
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
