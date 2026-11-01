// The JS/TS half of crates/djs-docs/src/highlight.rs, for the live examples' editor: the
// same rules and the same roles (tok-keyword, tok-string, ...), so code looks the same
// while it is edited as it does in the static page. A test in highlight.rs runs this file
// in node over the guide's examples and compares it with the Rust output (D2103).

const KW = new Set(
  ("as async await break case catch class const continue debugger default delete do else enum export " +
    "extends finally for from function get if implements import in instanceof interface let new of " +
    "return set static super switch this throw try type typeof var void while yield").split(" "),
);
// Constants: Shiki's `constant.language`, colored like numbers.
const CONSTS = new Set(["true", "false", "null", "undefined", "NaN", "Infinity"]);
const TYPES = new Set(["number", "string", "boolean", "bigint", "symbol", "object", "any", "unknown", "never"]);

// Rust's char::is_whitespace, is_alphabetic and is_alphanumeric.
const SPACE = /^\p{White_Space}$/u;
const ALPHA = /^\p{Alphabetic}$/u;
const ALNUM = /^[\p{Alphabetic}\p{N}]$/u;
const DIGIT = /^[0-9]$/;
const ASCII_ALNUM = /^[0-9A-Za-z]$/;

export function esc(s) {
  return s.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
}

/**
 * Splits `code` into `[role, text]` tokens (role null for whitespace); joined, the texts
 * are `code` again.
 */
export function tokens(code) {
  const c = Array.from(code);
  const out = [];
  let i = 0;
  let prevWord = "";
  let punct = "";
  const push = (role, s) => out.push([role, s]);
  while (i < c.length) {
    const ch = c[i];
    if (!SPACE.test(ch) && !ALNUM.test(ch) && !'_$/"\'`'.includes(ch)) {
      punct += ch;
      prevWord = "";
      i++;
      continue;
    }
    if (ch === "/" && c[i + 1] !== "/" && c[i + 1] !== "*") {
      punct += ch;
      i++;
      continue;
    }
    if (punct) {
      push("punctuation", punct);
      punct = "";
    }
    const rest = (j) => c.slice(i, j).join("");
    if (ch === "/" && c[i + 1] === "/") {
      let j = i;
      while (j < c.length && c[j] !== "\n") j++;
      push("comment", rest(j));
      i = j;
    } else if (ch === "/" && c[i + 1] === "*") {
      let j = i + 2;
      while (j + 1 < c.length && !(c[j] === "*" && c[j + 1] === "/")) j++;
      j = Math.min(j + 2, c.length);
      push("comment", rest(j));
      i = j;
    } else if (ch === '"' || ch === "`" || ch === "'") {
      let j = i + 1;
      while (j < c.length && c[j] !== ch) {
        if (c[j] === "\\") j++;
        j++;
      }
      j = Math.min(j + 1, c.length);
      push("string", rest(j));
      i = j;
    } else if (DIGIT.test(ch)) {
      let j = i;
      while (j < c.length && (ASCII_ALNUM.test(c[j]) || c[j] === "." || c[j] === "_")) j++;
      push("number", rest(j));
      i = j;
    } else if (ALPHA.test(ch) || ch === "_" || ch === "$") {
      let j = i + 1;
      while (j < c.length && (ALNUM.test(c[j]) || c[j] === "_" || c[j] === "$")) j++;
      const w = rest(j);
      let role;
      if (KW.has(w)) role = "keyword";
      else if (CONSTS.has(w)) role = "number";
      else if (TYPES.has(w) || /^[A-Z]/.test(w)) role = "type";
      else if (c[j] === "(" || prevWord === "fn" || prevWord === "function") role = "function";
      else role = "variable";
      push(role, w);
      prevWord = w;
      i = j;
    } else {
      // Whitespace (a run of it in one token).
      let j = i + 1;
      while (j < c.length && SPACE.test(c[j])) j++;
      push(null, rest(j));
      i = j;
    }
  }
  if (punct) push("punctuation", punct);
  return out;
}

/** The HTML highlight.rs makes of `code`. */
export function highlight(code) {
  return tokens(code)
    .map(([role, s]) => (role ? `<span class="tok-${role}">${esc(s)}</span>` : esc(s)))
    .join("");
}
