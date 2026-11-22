// A codemode body's leading import declarations (D3597), read on the host side: the
// engine skips them (they compile to nothing), and the Sandbox binds their names as the
// body's parameters to the host values its `modules` resolver names.

/** One name an import declaration binds. */
export interface ImportBinding {
  /** The export it imports: a name, `"default"`, or `"*"` for the namespace. */
  name: string;
  /** The local name. */
  local: string;
  /** Where the export's name is, for errors: a UTF-16 offset into the code. */
  at: number;
}

/** One import declaration (type-only ones are left out). */
export interface ImportDeclaration {
  specifier: string;
  /** Where the specifier is. */
  at: number;
  bindings: ImportBinding[];
}

/** A malformed import declaration, at `at`. */
export class ImportSyntaxError extends Error {
  readonly at: number;
  constructor(message: string, at: number) {
    super(message);
    this.at = at;
  }
}

type Token = { kind: "ident" | "string" | "punct" | "eof"; text: string; value: string; at: number; newline: boolean };

const ID_START = /[A-Za-z_$\u00aa-\uffff]/;
const ID_PART = /[\w$\u00aa-\uffff\u200c\u200d]/;

/** The tokens of an import prefix: identifiers, strings, punctuators; comments and spaces skipped. */
class Lexer {
  private i = 0;
  private readonly s: string;
  constructor(s: string) {
    this.s = s;
    // A hashbang line, as the engine allows.
    if (s.startsWith("#!")) {
      const n = s.indexOf("\n");
      this.i = n === -1 ? s.length : n;
    }
  }

  /** A copy, for looking ahead. */
  fork(): Lexer {
    return Object.assign(Object.create(Lexer.prototype) as Lexer, this);
  }

  next(): Token {
    const s = this.s;
    let newline = false;
    for (;;) {
      const c = s[this.i];
      if (c === undefined) return { kind: "eof", text: "", value: "", at: this.i, newline };
      if (c === "\n" || c === "\r" || c === "\u2028" || c === "\u2029") {
        newline = true;
        this.i++;
      } else if (/\s/.test(c)) this.i++;
      else if (s.startsWith("//", this.i)) {
        while (this.i < s.length && !/[\n\r\u2028\u2029]/.test(s[this.i])) this.i++;
      } else if (s.startsWith("/*", this.i)) {
        const end = s.indexOf("*/", this.i + 2);
        if (end === -1) throw new ImportSyntaxError("unterminated comment", this.i);
        if (/[\n\r\u2028\u2029]/.test(s.slice(this.i, end))) newline = true;
        this.i = end + 2;
      } else break;
    }
    const at = this.i;
    const c = s[at];
    if (c === '"' || c === "'") return { kind: "string", ...this.string(c), at, newline };
    if (ID_START.test(c) || c === "\\") {
      let j = at;
      while (j < s.length && (ID_PART.test(s[j]) || s[j] === "\\")) j++;
      const text = s.slice(at, j);
      this.i = j;
      // (An escaped identifier is not a keyword; the engine decides what it means.)
      return { kind: "ident", text, value: text.includes("\\") ? unescapeIdent(text, at) : text, at, newline };
    }
    this.i++;
    return { kind: "punct", text: c, value: c, at, newline };
  }

  private string(q: string): { text: string; value: string } {
    const s = this.s;
    const start = this.i;
    let j = start + 1;
    let value = "";
    for (;;) {
      const c = s[j];
      if (c === undefined || c === "\n" || c === "\r") throw new ImportSyntaxError("unterminated string", start);
      if (c === q) break;
      if (c === "\\") {
        const [v, n] = escape(s, j);
        value += v;
        j = n;
        continue;
      }
      value += c;
      j++;
    }
    this.i = j + 1;
    return { text: s.slice(start, j + 1), value };
  }
}

/** The escape at `s[j]` (a backslash): its value and the index after it. */
function escape(s: string, j: number): [string, number] {
  const c = s[j + 1];
  const simple: Record<string, string> = { n: "\n", t: "\t", r: "\r", b: "\b", f: "\f", v: "\v", "0": "\0" };
  if (c in simple && !(c === "0" && /[0-9]/.test(s[j + 2] ?? ""))) return [simple[c], j + 2];
  if (c === "x") {
    const h = s.slice(j + 2, j + 4);
    if (!/^[0-9a-fA-F]{2}$/.test(h)) throw new ImportSyntaxError("malformed escape", j);
    return [String.fromCharCode(parseInt(h, 16)), j + 4];
  }
  if (c === "u") {
    const m = /^u(?:([0-9a-fA-F]{4})|\{([0-9a-fA-F]{1,6})\})/.exec(s.slice(j + 1));
    if (!m) throw new ImportSyntaxError("malformed escape", j);
    return [String.fromCodePoint(parseInt(m[1] ?? m[2], 16)), j + 1 + m[0].length];
  }
  if (c === "\r" && s[j + 2] === "\n") return ["", j + 3];
  if (c === "\n" || c === "\r" || c === "\u2028" || c === "\u2029") return ["", j + 2];
  if (c === undefined || /[1-9]/.test(c)) throw new ImportSyntaxError("malformed escape", j);
  return [c, j + 2];
}

function unescapeIdent(text: string, at: number): string {
  let out = "";
  for (let j = 0; j < text.length; ) {
    if (text[j] === "\\") {
      if (text[j + 1] !== "u") throw new ImportSyntaxError("malformed escape", at + j);
      const [v, n] = escape(text, j);
      out += v;
      j = n;
    } else out += text[j++];
  }
  return out;
}

/**
 * The leading import declarations of codemode body `code`: those before its first other
 * statement (comments and blank lines may come between). `typescript`: `import type`
 * declarations and `type` specifiers are left out, as the compiler erases them. Throws an
 * `ImportSyntaxError` for a malformed one; the engine checks the syntax again.
 */
export function leadingImports(code: string, typescript = false): ImportDeclaration[] {
  const lx = new Lexer(code);
  const out: ImportDeclaration[] = [];
  let tok = lx.next();
  const fail = (why: string, t: Token = tok): never => {
    throw new ImportSyntaxError(why, t.at);
  };
  const is = (text: string, t: Token = tok) => t.kind === (/^\w/.test(text) ? "ident" : "punct") && t.text === text;
  const advance = () => (tok = lx.next());
  const peek = (): Token => lx.fork().next();
  /** The token after the next one. */
  const peek2 = (): Token => {
    const l = lx.fork();
    l.next();
    return l.next();
  };
  const name = (): Token => {
    if (tok.kind !== "ident" && tok.kind !== "string") fail("identifier expected");
    const t = tok;
    advance();
    return t;
  };
  const binding = (): Token => {
    if (tok.kind !== "ident") fail("identifier expected");
    const t = tok;
    advance();
    return t;
  };
  while (is("import")) {
    const next = peek();
    if (is("(", next) || is(".", next)) break;
    advance();
    if (tok.kind === "string") {
      const spec = tok;
      advance();
      end();
      out.push({ specifier: spec.value, at: spec.at, bindings: [] });
      continue;
    }
    // `import type X from`, `import type { .. }`, `import type * as` (not `import type from "m"`).
    let typeOnly = false;
    if (typescript && is("type")) {
      const n = peek();
      if (is("{", n) || is("*", n) || (n.kind === "ident" && n.text !== "from")) typeOnly = true;
      // `import type from from "m"`: a type-only default import named `from`.
      if (n.kind === "ident" && n.text === "from" && is("from", peek2())) typeOnly = true;
      if (typeOnly) advance();
    }
    const bindings: ImportBinding[] = [];
    let more = true;
    if (tok.kind === "ident" && !is("from")) {
      const t = binding();
      bindings.push({ name: "default", local: t.value, at: t.at });
      more = is(",");
      if (more) advance();
    } else if (tok.kind === "ident" && is("from")) {
      // `import from from "m"`: a default import named `from`.
      const n = peek();
      if (n.kind === "ident" && n.text === "from") {
        const t = binding();
        bindings.push({ name: "default", local: t.value, at: t.at });
        more = false;
      }
    }
    if (more) {
      if (is("*")) {
        advance();
        if (!is("as")) fail("expecting 'as'");
        advance();
        const t = binding();
        bindings.push({ name: "*", local: t.value, at: t.at });
      } else if (is("{")) {
        advance();
        while (!is("}")) {
          let skip = false;
          if (typescript && is("type")) {
            const n = peek();
            if (n.kind === "ident" || n.kind === "string") {
              // `{ type T }` and `{ type as }` (the type `as`) are erased; `{ type as x }` imports `type`.
              if (n.text === "as") skip = is(",", peek2()) || is("}", peek2());
              else skip = true;
            }
            if (skip) advance();
          }
          const imported = name();
          let local = imported;
          if (is("as")) {
            advance();
            local = binding();
          } else if (imported.kind === "string") fail("expecting 'as'");
          if (!skip) bindings.push({ name: imported.value, local: local.value, at: imported.at });
          if (!is(",")) break;
          advance();
        }
        if (!is("}")) fail("expecting '}'");
        advance();
      } else fail("expecting import clause");
    }
    if (!is("from")) fail("expecting 'from'");
    advance();
    // (`advance` moved `tok`, which the checker narrowed above.)
    const spec = tok as Token;
    if (spec.kind !== "string") fail("expecting string");
    advance();
    if (is("with") || (is("assert") && !tok.newline)) fail("import attributes are not supported in a codemode body");
    end();
    if (!typeOnly) out.push({ specifier: spec.value, at: spec.at, bindings });
  }
  return out;

  /** The declaration's end: `;`, a line break, the end, or `}`. */
  function end(): void {
    if (is(";")) {
      advance();
      return;
    }
    if (tok.kind === "eof" || tok.newline) return;
    fail("expecting ';'");
  }
}

/** The 1-based line and column of offset `at` in `code` (columns count code points). */
export function lineColumn(code: string, at: number): [number, number] {
  let line = 1;
  let start = 0;
  for (let i = 0; i < at && i < code.length; i++) {
    const c = code[i];
    if (c === "\n" || (c === "\r" && code[i + 1] !== "\n") || c === "\u2028" || c === "\u2029") {
      line++;
      start = i + 1;
    }
  }
  return [line, [...code.slice(start, at)].length + 1];
}
