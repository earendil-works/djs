// A small scanner of dump text (0216): enough JavaScript lexing to find brackets, top-level
// commas and statements in the exporter's output without being fooled by strings,
// templates, regexps and comments. Not a parser.

const REGEX_AFTER_WORD = new Set([
  "return", "typeof", "instanceof", "in", "of", "new", "delete", "void", "throw", "case", "do", "else", "yield", "await",
]);

/** Tokens: { s, e, t } with t the token text; strings, templates, regexps and comments opaque. */
export function tokens(text) {
  const out = [];
  let i = 0;
  const stack = []; // template nesting: brace depth at which a `${` opened
  let depth = 0;
  let prev = "";
  const push = (s, e) => {
    const t = text.slice(s, e);
    out.push({ s, e, t });
    if (!/^\s|^\/[/*]/.test(t)) prev = t;
  };
  const tmpl = (j) => {
    // j just after the opening backtick or a closing `}` of a substitution
    while (j < text.length) {
      const c = text[j];
      if (c === "\\") j += 2;
      else if (c === "`") return [j + 1, false];
      else if (c === "$" && text[j + 1] === "{") return [j + 2, true];
      else j++;
    }
    return [j, false];
  };
  while (i < text.length) {
    const c = text[i];
    if (/\s/.test(c)) {
      let j = i + 1;
      while (j < text.length && /\s/.test(text[j])) j++;
      push(i, j);
      i = j;
    } else if (c === "/" && text[i + 1] === "/") {
      let j = text.indexOf("\n", i);
      if (j < 0) j = text.length;
      push(i, j);
      i = j;
    } else if (c === "/" && text[i + 1] === "*") {
      const j = text.indexOf("*/", i + 2) + 2;
      push(i, j);
      i = j;
    } else if (c === '"' || c === "'") {
      let j = i + 1;
      while (j < text.length && text[j] !== c) j += text[j] === "\\" ? 2 : 1;
      push(i, j + 1);
      i = j + 1;
    } else if (c === "`") {
      const [j, sub] = tmpl(i + 1);
      push(i, j);
      if (sub) stack.push(depth);
      i = j;
    } else if (c === "}" && stack.length && stack[stack.length - 1] === depth) {
      stack.pop();
      const [j, sub] = tmpl(i + 1);
      push(i, j);
      if (sub) stack.push(depth);
      i = j;
    } else if (c === "/" && (prev === "" || /[(,=:[!&|?{};+\-*%<>~^]$/.test(prev) || REGEX_AFTER_WORD.has(prev))) {
      let j = i + 1;
      let cls = false;
      while (j < text.length && (cls || text[j] !== "/")) {
        if (text[j] === "\\") j++;
        else if (text[j] === "[") cls = true;
        else if (text[j] === "]") cls = false;
        j++;
      }
      j++;
      while (j < text.length && /[a-z]/i.test(text[j])) j++;
      push(i, j);
      i = j;
    } else if (/[A-Za-z0-9_$\u0080-\uffff]/.test(c)) {
      let j = i + 1;
      while (j < text.length && /[A-Za-z0-9_$.\u0080-\uffff]/.test(text[j]) && !(text[j] === "." && text[j + 1] === ".")) {
        // numbers and member chains as one token (a.b.c), but not `...`
        if (text[j] === "." && !/[A-Za-z0-9_$]/.test(text[j + 1] ?? "")) break;
        j++;
      }
      push(i, j);
      i = j;
    } else {
      if (c === "{" || c === "(" || c === "[") depth++;
      if (c === "}" || c === ")" || c === "]") depth--;
      let j = i + 1;
      if (c === "." && text[i + 1] === "." && text[i + 2] === ".") j = i + 3;
      else if (c === "=" && text[i + 1] === ">") j = i + 2;
      push(i, j);
      i = j;
    }
  }
  return out;
}

/** The bracket depth change of a token (template pieces open and close substitutions). */
export function delta(t) {
  if (t === "(" || t === "[" || t === "{") return 1;
  if (t === ")" || t === "]" || t === "}") return -1;
  if (t.length > 1 && t[0] === "`") return t.endsWith("${") ? 1 : 0;
  if (t.length > 1 && t[0] === "}") return t.endsWith("${") ? 0 : -1;
  return 0;
}

/** Index of the bracket matching the one at `open` (text offset). */
export function matchClose(text, open, toks = tokens(text)) {
  let k = toks.findIndex((t) => t.s === open);
  if (k < 0) throw new Error(`no token at ${open}`);
  let d = 0;
  for (; k < toks.length; k++) {
    d += delta(toks[k].t);
    if (d === 0) return toks[k].s;
  }
  throw new Error(`unbalanced from ${open}`);
}

/** Top-level comma-separated parts of text[s, e) as [start, end) pairs. */
export function splitTop(text, s, e, toks = tokens(text)) {
  const parts = [];
  let d = 0;
  let start = s;
  for (const t of toks) {
    if (t.s < s || t.s >= e) continue;
    if (t.t === "," && d === 0) {
      parts.push([start, t.s]);
      start = t.e;
    }
    d += delta(t.t);
  }
  parts.push([start, e]);
  return parts;
}

/**
 * Top-level statements: [start, end) spans ending after their `;` or `}` and the newline
 * that follows at depth 0.
 */
export function statements(text, toks = tokens(text)) {
  const out = [];
  let d = 0;
  let start = 0;
  for (const t of toks) {
    d += delta(t.t);
    if (d === 0 && /^\s*\n/.test(t.t)) {
      const prevChar = text[t.s - 1];
      if (prevChar === ";" || prevChar === "}") {
        const e = t.s + t.t.lastIndexOf("\n") + 1;
        out.push([start, e]);
        start = e;
      }
    }
  }
  if (start < text.length) out.push([start, text.length]);
  return out;
}
