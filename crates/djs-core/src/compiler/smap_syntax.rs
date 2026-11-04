//! The code around a function's suspension sites (D2850): what the plain-dump exporter
//! needs to write "the rest of the function from a site" as source text (a continuation,
//! D2830, D2831) without parsing the function again.
//!
//! The compiler is single-pass and builds no syntax tree. While recording suspension
//! maps it keeps every syntax node that *encloses a site* of the function (statements
//! and expressions of the function's own code, not of nested functions), with the spans
//! of its parts, and for each site the path of those nodes from the function's body down
//! to the site. Everything else stays text: the exporter copies spans of the function's
//! source verbatim (`FunctionMap::start` + offset).
//!
//! Conventions:
//!
//! - Every offset is from the function's start (`FunctionMap::start`), like `Site::pos`
//!   and the offsets in slot names (`call.callee@41`); spans are half-open `[start,
//!   end)`. Offsets are into the compiled source, TypeScript annotations included.
//! - A node's `parts` are its direct parts in source order, each with its role. A part
//!   that is absent from the source (no `else`, an empty `for` test) is not listed.
//!   Parts never overlap; the text between them is punctuation and keywords (`if (`,
//!   `) `, `;`, `=>`), which the exporter writes itself.
//! - A part's span is the span of the construct in it, parentheses around it included:
//!   in `f((await x))` the argument part is `(await x)`.
//! - Where a slot name refers to a construct, the offset is the node's `start` or a
//!   part's `start` (see each [`NodeKind`]); so a temporary `c@o` of a site belongs to the
//!   node on the site's path whose start or part start is `o`.
//!
//! Only these constructs are nodes; destructuring patterns, class bodies and TypeScript
//! expressions (`x as T`, `x!`) are not (yet). A path that passes through one has a
//! **gap**: the part that should hold the next node holds more than that node and
//! parentheses. `Site::gap` reports it, and the exporter must refuse such a site rather
//! than drop the construct.

use alloc::string::String;
use alloc::vec::Vec;

/// The syntax recorded for one function (`FunctionMap::syntax`).
#[derive(Clone, Debug, Default)]
pub struct Syntax {
    /// The nodes that enclose a site, in no particular order: sites refer to them by
    /// index (`Step::node`).
    pub nodes: Vec<Node>,
    /// References in the function's own code, and in the arrow functions nested in it
    /// (which share them), to `arguments`, `new.target` and `super`: a continuation is
    /// a function of its own, so its `arguments` is not the original's (the exporter
    /// renames these to a binding of the saved value), and it has no `new.target` or
    /// home object. Source order.
    pub special: Vec<(Special, u32)>,
}

/// What a [`Syntax::special`] reference is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Special {
    /// The identifier `arguments`, resolving to this function's arguments object
    /// (offset of the identifier).
    Arguments,
    /// `new.target` (offset of `new`).
    NewTarget,
    /// `super` in `super.x`, `super[x]` or `super(...)` (offset of `super`).
    Super,
}

/// One step of a site's path (`Site::path`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Step {
    /// Index into `Syntax::nodes`.
    pub node: u32,
    /// The index (into the node's `parts`) of the part that holds the next step's node;
    /// `None` for the last step (the site's own construct, see `Site::path`).
    pub part: Option<u16>,
}

/// A syntax node that encloses at least one site.
#[derive(Clone, Debug)]
pub struct Node {
    pub kind: NodeKind,
    pub start: u32,
    pub end: u32,
    pub parts: Vec<Part>,
    /// The bindings of the scopes this node opens, in declaration order: a block's or
    /// case block's lexical declarations, a function body's parameters, `var`s,
    /// body-level declarations and own name, a `for` head's, a `catch` parameter's.
    /// Empty for nodes that open no scope.
    pub decls: Vec<Decl>,
}

/// A part of a node: its role and its span.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Part {
    pub role: Role,
    pub start: u32,
    pub end: u32,
}

/// A binding a node's scope declares.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Decl {
    pub name: String,
    pub kind: DeclKind,
    /// The offset that names the declaring scope in suspension maps (`Slot::Binding`'s
    /// `scope`, the `S` of a shadowed binding's key `name@S`).
    pub scope: u32,
    /// The part of the node whose code declares it (a statement of a statement list,
    /// a `for` head, the parameter list, a `catch` parameter); `None` for a named
    /// function expression's own name. A binding declared by a statement before the
    /// site's statement has run its declaration (its value is in the frame, a TDZ value
    /// if it is still uninitialized); one declared after it has not.
    pub part: Option<u16>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeclKind {
    Var,
    Let,
    Const,
    /// `using` and `await using` declarations.
    Using,
    AwaitUsing,
    /// A class declaration's name.
    Class,
    /// A function declaration's name (initialized at its scope's entry).
    Function,
    Param,
    CatchParam,
    /// A named function expression's own name.
    FuncName,
}

/// The role of a part in its node.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    // ---- statements
    /// A function's parameter list, with its parentheses when it has them.
    Params,
    /// A statement of a statement list (function body, block, case clause).
    Stmt,
    /// A function declaration in a statement list: hoisted, it runs nothing where it
    /// stands.
    FuncDecl,
    /// A loop's or labelled statement's body.
    Body,
    /// The test of `if`, `while`, `do`, `for` and `?:`.
    Test,
    /// The branches of `if` and `?:`.
    Then,
    Else,
    /// A `for (init; …)`'s init; a declarator's initializer (after the `=`).
    Init,
    /// A `for (…; …; update)`'s update.
    Update,
    /// A `for-in`/`for-of`'s left side (`const [k, v]`, `x.y`).
    Head,
    /// A `for-in`/`for-of`'s right side.
    Iterable,
    /// A label's identifier.
    Label,
    /// A `try`'s block, `catch` parameter (without its parentheses), `catch` block and
    /// `finally` block.
    Block,
    CatchParam,
    CatchBody,
    Finally,
    /// A `switch`'s discriminant; a `case`'s test (followed by the clause's statements
    /// as `Stmt` parts); a `default` (the keyword's span, followed by its statements).
    Discriminant,
    Case,
    Default,
    // ---- expressions
    /// The operand of `await`, `yield`, `yield*`, a unary or update operator, `return`
    /// and `throw`; the expression of an expression statement; the concise body of an
    /// arrow function.
    Operand,
    /// An operator token (binary, logical, assignment, unary, update).
    Op,
    /// The operands of a binary, logical or assignment operator (`Left` is the target of
    /// an assignment).
    Left,
    Right,
    /// A declarator's target (a name or a pattern).
    Target,
    /// A call's or `new`'s callee (`super` for `super(...)`), the `(` of its arguments
    /// (`?.(` for an optional call: the whole token), and each argument (`...x` is
    /// `Spread`). An argument part starts where `call.arg@`/`new.arg@` name it; the
    /// argument array of a call with spread is named by the `(`'s start.
    Callee,
    Open,
    Arg,
    Spread,
    /// A member expression's object (`super` for `super.x`) and its property: the
    /// name of `.x` (or `?.x`), or for a computed member the whole `[key]`, brackets
    /// included.
    Object,
    Property,
    /// An array literal's element (`...x` is `Spread`), or a hole (an empty span where
    /// the hole is).
    Element,
    Hole,
    /// A template literal's text part as its token: the backtick or `}` before it to the
    /// `${` or backtick after it (`template.part@` names it by its start), and a
    /// substitution's expression.
    Quasi,
    Subst,
    /// A tagged template's tag.
    Tag,
    /// An object literal's property: a plain key (identifier, string, number), a
    /// computed key with its brackets (`object.key@` names it by the `[`), the value
    /// after the `:`, a shorthand property `x`, a method, getter or setter as a whole,
    /// and a spread `...x` (`Spread`).
    Key,
    ComputedKey,
    Value,
    Shorthand,
    Method,
    /// An expression of a comma sequence.
    Expr,
}

/// What a node is, and what parts it has (in this order; `?` optional, `*` repeated).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NodeKind {
    // ---- statements
    /// The function itself: its span is the whole function `[0, end - start)`. Parts:
    /// `Params?` then the body's statements (`Stmt`/`FuncDecl`*), or for an arrow with
    /// a concise body `Params? Operand`. The codemode body and Scripts have no
    /// `Params`. Always `path[0]`.
    Body,
    /// `{ … }`: `Stmt`/`FuncDecl`*. Also the blocks of `try`, `catch`, `finally` when a
    /// path enters them.
    Block,
    /// An expression statement: `Operand`.
    Expr,
    /// A `var`/`let`/`const`/`using`/`await using` declaration (a statement, or the
    /// declaration in a `for` init): `(Target Init?)*`.
    Decl(DeclKind),
    /// `if (Test) Then else Else?`.
    If,
    /// `Label: Body`.
    Label,
    /// `while (Test) Body`, `do Body while (Test)`.
    While,
    DoWhile,
    /// `for (Init?; Test?; Update?) Body`.
    For,
    /// `for (Head in Iterable) Body`, `for (Head of Iterable) Body`,
    /// `for await (Head of Iterable) Body`. Their temporaries (`for-of.iterator@…`) are
    /// named by the node's start.
    ForIn,
    ForOf,
    ForAwait,
    /// `try Block catch (CatchParam?) CatchBody? finally Finally?`. Its temporaries
    /// (`completion.kind@…`) are named by the node's start.
    Try,
    /// `switch (Discriminant) { (Case|Default Stmt*)* }`.
    Switch,
    /// `return Operand?`, `throw Operand`.
    Return,
    Throw,
    // ---- expressions
    /// `await Operand`, `yield Operand?`, `yield* Operand`.
    Await,
    Yield,
    YieldStar,
    /// `Callee Open Arg*` (args `Arg` or `Spread`); `call.callee@`/`call.this@` name it
    /// by its start. `optional`: this call is `?.(`.
    Call { optional: bool },
    /// `new Callee Open? Arg*`; `new.callee@` names it by its start.
    New,
    /// `import(Arg Arg?)`.
    ImportCall,
    /// `Object Property`; `member.object@` names it by its start. `optional`: this
    /// member is `?.`.
    Member { computed: bool, optional: bool },
    /// `Left Op Right`: binary operators (`binary.left@` by its start), `&&`/`||`/`??`,
    /// assignments (`assign.*@` by its start).
    Binary,
    Logical,
    Assign,
    /// `Op Operand` (`typeof`, `!`, `-`, `delete`, …).
    Unary,
    /// `Op Operand` or `Operand Op` (`++`/`--`).
    Update,
    /// `Test ? Then : Else`.
    Conditional,
    /// `Expr (, Expr)*`.
    Sequence,
    /// `` `…${Subst}…` ``: `Quasi (Subst Quasi)*`.
    Template,
    /// `Tag Quasi (Subst Quasi)*`; its arguments are named like a call's.
    TaggedTemplate,
    /// `[ Element|Hole|Spread * ]`; `array.literal@` names it by its start (the `[`).
    Array,
    /// `{ (Key Value | ComputedKey Value | Shorthand | Method | Spread)* }`;
    /// `object.literal@` names it by its start (the `{`).
    Object,
}
