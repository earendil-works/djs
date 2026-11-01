//! Token kinds, keyword ids and the token record.

/// What a token is. Reserved words have their own kinds; contextual keywords
/// (`let`, `async`, `of`, the TypeScript ones, ...) are [`TokenKind::Ident`] with a
/// [`Kw`] id in [`Token::kw`].
///
/// `>` is always a single [`TokenKind::Gt`]; the parser calls
/// [`Lexer::rescan_greater`](super::Lexer::rescan_greater) where a longer operator is
/// allowed. `/` and `/=` become regex literals through
/// [`Lexer::rescan_as_regex`](super::Lexer::rescan_as_regex), and `}` becomes a template
/// continuation through
/// [`Lexer::rescan_template_continuation`](super::Lexer::rescan_template_continuation).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TokenKind {
    Eof,

    // Names and literals.
    /// An identifier name that is not an unescaped reserved word. Escaped reserved words
    /// are `Ident` too, with the [`Token::HAS_ESCAPE`] flag and a reserved [`Kw`].
    Ident,
    /// `#name`. The span includes the `#`.
    PrivateName,
    Number,
    BigInt,
    String,
    /// `` `...` `` (a template without substitutions).
    NoSubstTemplate,
    /// `` `...${ ``
    TemplateHead,
    /// `}...${`
    TemplateMiddle,
    /// `` }...` ``
    TemplateTail,
    /// `/body/flags`
    Regex,

    // Punctuators.
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Dot,
    Ellipsis,
    Semicolon,
    Comma,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
    StrictEq,
    StrictNe,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    StarStar,
    PlusPlus,
    MinusMinus,
    Shl,
    Shr,
    UShr,
    Amp,
    Pipe,
    Caret,
    Bang,
    Tilde,
    AmpAmp,
    PipePipe,
    QuestionQuestion,
    Question,
    QuestionDot,
    Colon,
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    PercentAssign,
    StarStarAssign,
    ShlAssign,
    ShrAssign,
    UShrAssign,
    AmpAssign,
    PipeAssign,
    CaretAssign,
    AmpAmpAssign,
    PipePipeAssign,
    QuestionQuestionAssign,
    Arrow,
    At,

    // Reserved words (always reserved; the engine is always strict, but the strict-only
    // reserved words such as `let` and `yield` are contextual, see `Kw`).
    Break,
    Case,
    Catch,
    Class,
    Const,
    Continue,
    Debugger,
    Default,
    Delete,
    Do,
    Else,
    Enum,
    Export,
    Extends,
    False,
    Finally,
    For,
    Function,
    If,
    Import,
    In,
    Instanceof,
    New,
    Null,
    Return,
    Super,
    Switch,
    This,
    Throw,
    True,
    Try,
    Typeof,
    Var,
    Void,
    While,
    With,
}

impl TokenKind {
    /// Reserved words, which are valid as an `IdentifierName` (property names, ...).
    pub fn is_reserved_word(self) -> bool {
        (self as u8) >= (TokenKind::Break as u8)
    }

    /// `Ident` or a reserved word: anything valid as an `IdentifierName`.
    pub fn is_identifier_name(self) -> bool {
        self == TokenKind::Ident || self.is_reserved_word()
    }

    /// Assignment operators, `=` included.
    pub fn is_assign_op(self) -> bool {
        use TokenKind::*;
        matches!(
            self,
            Assign
                | PlusAssign
                | MinusAssign
                | StarAssign
                | SlashAssign
                | PercentAssign
                | StarStarAssign
                | ShlAssign
                | ShrAssign
                | UShrAssign
                | AmpAssign
                | PipeAssign
                | CaretAssign
                | AmpAmpAssign
                | PipePipeAssign
                | QuestionQuestionAssign
        )
    }
}

/// Keyword id of an identifier name: every reserved word plus the contextual keywords
/// the parser (and later the TypeScript type skipper) needs to recognize quickly.
///
/// A token's `kw` is set for [`TokenKind::Ident`], for reserved word kinds and for
/// [`TokenKind::PrivateName`] (for the part after `#`). It is computed from the cooked
/// name, so `l\u0065t` has `Kw::Let`; check [`Token::HAS_ESCAPE`] before treating a
/// token as a keyword.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Kw {
    None,
    // Reserved words, in the same order as in `TokenKind`.
    Break,
    Case,
    Catch,
    Class,
    Const,
    Continue,
    Debugger,
    Default,
    Delete,
    Do,
    Else,
    Enum,
    Export,
    Extends,
    False,
    Finally,
    For,
    Function,
    If,
    Import,
    In,
    Instanceof,
    New,
    Null,
    Return,
    Super,
    Switch,
    This,
    Throw,
    True,
    Try,
    Typeof,
    Var,
    Void,
    While,
    With,
    // Contextual keywords (identifiers in most positions).
    // `let static yield implements interface package private protected public` are
    // reserved in strict code: see `Kw::is_strict_reserved`.
    Let,
    Static,
    Yield,
    Await,
    Async,
    Of,
    Get,
    Set,
    Target,
    Meta,
    As,
    From,
    Package,
    Eval,
    Arguments,
    Constructor,
    Using,
    Accessor,
    Type,
    Interface,
    Declare,
    Abstract,
    Readonly,
    Namespace,
    Module,
    Keyof,
    Infer,
    Is,
    Asserts,
    Satisfies,
    Override,
    Implements,
    Private,
    Protected,
    Public,
    Unique,
    Global,
    Out,
}

impl Kw {
    /// Looks up the keyword id of a cooked identifier name.
    pub fn lookup(s: &[u8]) -> Kw {
        // Fixed-length slice patterns compile to a length switch plus a byte-wise
        // decision tree, so this is a handful of comparisons.
        match s {
            [b'a', b's'] => Kw::As,
            [b'd', b'o'] => Kw::Do,
            [b'i', b'f'] => Kw::If,
            [b'i', b'n'] => Kw::In,
            [b'i', b's'] => Kw::Is,
            [b'o', b'f'] => Kw::Of,
            [b'f', b'o', b'r'] => Kw::For,
            [b'g', b'e', b't'] => Kw::Get,
            [b'l', b'e', b't'] => Kw::Let,
            [b'n', b'e', b'w'] => Kw::New,
            [b'o', b'u', b't'] => Kw::Out,
            [b's', b'e', b't'] => Kw::Set,
            [b't', b'r', b'y'] => Kw::Try,
            [b'v', b'a', b'r'] => Kw::Var,
            [b'c', b'a', b's', b'e'] => Kw::Case,
            [b'e', b'l', b's', b'e'] => Kw::Else,
            [b'e', b'n', b'u', b'm'] => Kw::Enum,
            [b'e', b'v', b'a', b'l'] => Kw::Eval,
            [b'f', b'r', b'o', b'm'] => Kw::From,
            [b'm', b'e', b't', b'a'] => Kw::Meta,
            [b'n', b'u', b'l', b'l'] => Kw::Null,
            [b't', b'h', b'i', b's'] => Kw::This,
            [b't', b'r', b'u', b'e'] => Kw::True,
            [b't', b'y', b'p', b'e'] => Kw::Type,
            [b'v', b'o', b'i', b'd'] => Kw::Void,
            [b'w', b'i', b't', b'h'] => Kw::With,
            [b'a', b's', b'y', b'n', b'c'] => Kw::Async,
            [b'a', b'w', b'a', b'i', b't'] => Kw::Await,
            [b'b', b'r', b'e', b'a', b'k'] => Kw::Break,
            [b'c', b'a', b't', b'c', b'h'] => Kw::Catch,
            [b'c', b'l', b'a', b's', b's'] => Kw::Class,
            [b'c', b'o', b'n', b's', b't'] => Kw::Const,
            [b'f', b'a', b'l', b's', b'e'] => Kw::False,
            [b'i', b'n', b'f', b'e', b'r'] => Kw::Infer,
            [b'k', b'e', b'y', b'o', b'f'] => Kw::Keyof,
            [b's', b'u', b'p', b'e', b'r'] => Kw::Super,
            [b't', b'h', b'r', b'o', b'w'] => Kw::Throw,
            [b'u', b's', b'i', b'n', b'g'] => Kw::Using,
            [b'w', b'h', b'i', b'l', b'e'] => Kw::While,
            [b'y', b'i', b'e', b'l', b'd'] => Kw::Yield,
            [b'd', b'e', b'l', b'e', b't', b'e'] => Kw::Delete,
            [b'e', b'x', b'p', b'o', b'r', b't'] => Kw::Export,
            [b'g', b'l', b'o', b'b', b'a', b'l'] => Kw::Global,
            [b'i', b'm', b'p', b'o', b'r', b't'] => Kw::Import,
            [b'm', b'o', b'd', b'u', b'l', b'e'] => Kw::Module,
            [b'p', b'u', b'b', b'l', b'i', b'c'] => Kw::Public,
            [b'r', b'e', b't', b'u', b'r', b'n'] => Kw::Return,
            [b's', b't', b'a', b't', b'i', b'c'] => Kw::Static,
            [b's', b'w', b'i', b't', b'c', b'h'] => Kw::Switch,
            [b't', b'a', b'r', b'g', b'e', b't'] => Kw::Target,
            [b't', b'y', b'p', b'e', b'o', b'f'] => Kw::Typeof,
            [b'u', b'n', b'i', b'q', b'u', b'e'] => Kw::Unique,
            [b'a', b's', b's', b'e', b'r', b't', b's'] => Kw::Asserts,
            [b'd', b'e', b'c', b'l', b'a', b'r', b'e'] => Kw::Declare,
            [b'd', b'e', b'f', b'a', b'u', b'l', b't'] => Kw::Default,
            [b'e', b'x', b't', b'e', b'n', b'd', b's'] => Kw::Extends,
            [b'f', b'i', b'n', b'a', b'l', b'l', b'y'] => Kw::Finally,
            [b'p', b'a', b'c', b'k', b'a', b'g', b'e'] => Kw::Package,
            [b'p', b'r', b'i', b'v', b'a', b't', b'e'] => Kw::Private,
            [b'a', b'b', b's', b't', b'r', b'a', b'c', b't'] => Kw::Abstract,
            [b'a', b'c', b'c', b'e', b's', b's', b'o', b'r'] => Kw::Accessor,
            [b'c', b'o', b'n', b't', b'i', b'n', b'u', b'e'] => Kw::Continue,
            [b'd', b'e', b'b', b'u', b'g', b'g', b'e', b'r'] => Kw::Debugger,
            [b'f', b'u', b'n', b'c', b't', b'i', b'o', b'n'] => Kw::Function,
            [b'o', b'v', b'e', b'r', b'r', b'i', b'd', b'e'] => Kw::Override,
            [b'r', b'e', b'a', b'd', b'o', b'n', b'l', b'y'] => Kw::Readonly,
            [b'a', b'r', b'g', b'u', b'm', b'e', b'n', b't', b's'] => Kw::Arguments,
            [b'i', b'n', b't', b'e', b'r', b'f', b'a', b'c', b'e'] => Kw::Interface,
            [b'n', b'a', b'm', b'e', b's', b'p', b'a', b'c', b'e'] => Kw::Namespace,
            [b'p', b'r', b'o', b't', b'e', b'c', b't', b'e', b'd'] => Kw::Protected,
            [b's', b'a', b't', b'i', b's', b'f', b'i', b'e', b's'] => Kw::Satisfies,
            [b'i', b'm', b'p', b'l', b'e', b'm', b'e', b'n', b't', b's'] => Kw::Implements,
            [b'i', b'n', b's', b't', b'a', b'n', b'c', b'e', b'o', b'f'] => Kw::Instanceof,
            [
                b'c',
                b'o',
                b'n',
                b's',
                b't',
                b'r',
                b'u',
                b'c',
                b't',
                b'o',
                b'r',
            ] => Kw::Constructor,
            _ => Kw::None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Kw::None => "",
            Kw::Break => "break",
            Kw::Case => "case",
            Kw::Catch => "catch",
            Kw::Class => "class",
            Kw::Const => "const",
            Kw::Continue => "continue",
            Kw::Debugger => "debugger",
            Kw::Default => "default",
            Kw::Delete => "delete",
            Kw::Do => "do",
            Kw::Else => "else",
            Kw::Enum => "enum",
            Kw::Export => "export",
            Kw::Extends => "extends",
            Kw::False => "false",
            Kw::Finally => "finally",
            Kw::For => "for",
            Kw::Function => "function",
            Kw::If => "if",
            Kw::Import => "import",
            Kw::In => "in",
            Kw::Instanceof => "instanceof",
            Kw::New => "new",
            Kw::Null => "null",
            Kw::Return => "return",
            Kw::Super => "super",
            Kw::Switch => "switch",
            Kw::This => "this",
            Kw::Throw => "throw",
            Kw::True => "true",
            Kw::Try => "try",
            Kw::Typeof => "typeof",
            Kw::Var => "var",
            Kw::Void => "void",
            Kw::While => "while",
            Kw::With => "with",
            Kw::Let => "let",
            Kw::Static => "static",
            Kw::Yield => "yield",
            Kw::Await => "await",
            Kw::Async => "async",
            Kw::Of => "of",
            Kw::Get => "get",
            Kw::Set => "set",
            Kw::Target => "target",
            Kw::Meta => "meta",
            Kw::As => "as",
            Kw::From => "from",
            Kw::Package => "package",
            Kw::Eval => "eval",
            Kw::Arguments => "arguments",
            Kw::Constructor => "constructor",
            Kw::Using => "using",
            Kw::Accessor => "accessor",
            Kw::Type => "type",
            Kw::Interface => "interface",
            Kw::Declare => "declare",
            Kw::Abstract => "abstract",
            Kw::Readonly => "readonly",
            Kw::Namespace => "namespace",
            Kw::Module => "module",
            Kw::Keyof => "keyof",
            Kw::Infer => "infer",
            Kw::Is => "is",
            Kw::Asserts => "asserts",
            Kw::Satisfies => "satisfies",
            Kw::Override => "override",
            Kw::Implements => "implements",
            Kw::Private => "private",
            Kw::Protected => "protected",
            Kw::Public => "public",
            Kw::Unique => "unique",
            Kw::Global => "global",
            Kw::Out => "out",
        }
    }

    /// The token kind of a reserved word, `None` for contextual keywords.
    pub fn reserved_kind(self) -> Option<TokenKind> {
        Some(match self {
            Kw::Break => TokenKind::Break,
            Kw::Case => TokenKind::Case,
            Kw::Catch => TokenKind::Catch,
            Kw::Class => TokenKind::Class,
            Kw::Const => TokenKind::Const,
            Kw::Continue => TokenKind::Continue,
            Kw::Debugger => TokenKind::Debugger,
            Kw::Default => TokenKind::Default,
            Kw::Delete => TokenKind::Delete,
            Kw::Do => TokenKind::Do,
            Kw::Else => TokenKind::Else,
            Kw::Enum => TokenKind::Enum,
            Kw::Export => TokenKind::Export,
            Kw::Extends => TokenKind::Extends,
            Kw::False => TokenKind::False,
            Kw::Finally => TokenKind::Finally,
            Kw::For => TokenKind::For,
            Kw::Function => TokenKind::Function,
            Kw::If => TokenKind::If,
            Kw::Import => TokenKind::Import,
            Kw::In => TokenKind::In,
            Kw::Instanceof => TokenKind::Instanceof,
            Kw::New => TokenKind::New,
            Kw::Null => TokenKind::Null,
            Kw::Return => TokenKind::Return,
            Kw::Super => TokenKind::Super,
            Kw::Switch => TokenKind::Switch,
            Kw::This => TokenKind::This,
            Kw::Throw => TokenKind::Throw,
            Kw::True => TokenKind::True,
            Kw::Try => TokenKind::Try,
            Kw::Typeof => TokenKind::Typeof,
            Kw::Var => TokenKind::Var,
            Kw::Void => TokenKind::Void,
            Kw::While => TokenKind::While,
            Kw::With => TokenKind::With,
            _ => return None,
        })
    }

    /// Reserved in strict mode code (ECMA-262 12.7.2), in addition to the reserved words.
    pub fn is_strict_reserved(self) -> bool {
        matches!(
            self,
            Kw::Let
                | Kw::Static
                | Kw::Yield
                | Kw::Implements
                | Kw::Interface
                | Kw::Package
                | Kw::Private
                | Kw::Protected
                | Kw::Public
        )
    }
}

/// A token: kind, keyword id, flags and the byte span `start..end` in the source.
///
/// Values (names, cooked strings, numbers) are not stored; the parser asks the lexer
/// for them from the span (see `Lexer::ident_value`, `Lexer::cook_string`, ...).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub kw: Kw,
    pub flags: u8,
    pub start: u32,
    pub end: u32,
}

impl Token {
    /// A line terminator (or a multi-line comment containing one) precedes the token.
    pub const NEWLINE_BEFORE: u8 = 1 << 0;
    /// The value differs from the source text: an identifier or private name with a
    /// `\u` escape, a string with a `\`, a template part with a `\` or a CR.
    pub const HAS_ESCAPE: u8 = 1 << 1;
    /// A template part contains a CR, so its raw value needs CR/CRLF normalization.
    pub const HAS_CR: u8 = 1 << 2;
    /// A template part contains an invalid escape: its cooked value is `undefined`
    /// (valid only in tagged templates).
    pub const INVALID_ESCAPE: u8 = 1 << 3;
    /// A numeric literal contains `_` separators.
    pub const HAS_SEPARATOR: u8 = 1 << 4;

    pub fn newline_before(&self) -> bool {
        self.flags & Self::NEWLINE_BEFORE != 0
    }

    pub fn has_escape(&self) -> bool {
        self.flags & Self::HAS_ESCAPE != 0
    }

    pub fn has_flag(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }

    pub fn span(&self) -> Span {
        Span {
            start: self.start,
            end: self.end,
        }
    }

    /// The unescaped contextual keyword `kw` (`async`, `of`, `get`, ...).
    pub fn is_contextual(&self, kw: Kw) -> bool {
        self.kind == TokenKind::Ident && self.kw == kw && !self.has_escape()
    }

    /// An identifier whose cooked name is a reserved word (`\u0069f`). Valid as an
    /// `IdentifierName`, but never as an identifier reference or binding.
    pub fn is_escaped_reserved(&self) -> bool {
        self.kind == TokenKind::Ident && self.kw.reserved_kind().is_some()
    }
}

/// A byte range `start..end` in the source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn len(&self) -> u32 {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// The bytes of this span. Out-of-range spans give an empty slice.
    pub fn slice<'a>(&self, src: &'a [u8]) -> &'a [u8] {
        src.get(self.start as usize..self.end as usize)
            .unwrap_or(&[])
    }
}
