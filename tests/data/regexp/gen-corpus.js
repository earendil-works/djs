// Generates corpus.txt: regexp compile and exec results computed by QuickJS.
// Run ./gen-corpus.sh (uses ~/Development/dquickts/qjs). `qjs gen-corpus.js SEED SCALE`
// makes a different, larger corpus for out-of-tree runs.
//
// One JSON array per line:
//   [pattern, flags, "!SyntaxError: message"]          the pattern does not compile
//   [pattern, flags, input, lastIndex, result]          result: null (no match) or the
//                                                       [start, end] (or null) of each group
// exec starts at lastIndex for g/y patterns and at 0 otherwise, as RegExp.prototype.exec.

const args = typeof scriptArgs !== "undefined" ? scriptArgs.slice(1) : [];
const SCALE = args.length > 1 ? Number(args[1]) : 1;
let seed = args.length > 0 ? Number(args[0]) | 0 : 0x13579bdf;
function rand32() {
  seed = (seed + 0x6d2b79f5) | 0;
  let t = seed;
  t = Math.imul(t ^ (t >>> 15), t | 1);
  t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
  return (t ^ (t >>> 14)) >>> 0;
}
const below = (n) => rand32() % n;
const pick = (a) => a[below(a.length)];

const lines = [];
function run(pattern, flags, inputs) {
  let re;
  try {
    re = new RegExp(pattern, flags + "d");
  } catch (e) {
    lines.push(JSON.stringify([pattern, flags, "!" + e.name + ": " + e.message]));
    return;
  }
  for (const input of inputs) {
    const starts = flags.includes("g") || flags.includes("y") ? [0, below(input.length + 2)] : [0];
    for (const li of starts) {
      re.lastIndex = li;
      const r = re.exec(input);
      const res = r === null ? null : r.indices.map((x) => (x === undefined ? null : x));
      lines.push(JSON.stringify([pattern, flags, input, li, res]));
    }
  }
}

// ---- curated patterns ----
const texts = [
  "", "a", "abc", "aaa", "abcabc", "Hello, World!", "foo bar  baz", "  trim me  ",
  "john.doe@example.com, jane@test.org", "https://example.com/path?q=1&r=two#frag",
  "2024-01-15T10:30:00Z", "call 555-123-4567 or (555) 987-6543", "a1b2c3", "x=1; y=22; z=333",
  "line one\nline two\r\nline three", "tab\tsep\tvalues", "CamelCaseString", "snake_case_name",
  "<div class=\"a\">text</div>", "{{name}} and {{ other }}", "$12.50, $3", "ÄÖÜäöüß", "ǅǆǄ",
  "Straße STRASSE", "ſ s S K k \u212a", "😀 smile 😀😀", "\u{10400}\u{10428}", "a\u0000b",
  "aaaaaaaaaab", "abab", "ababab", "The quick brown fox", "1,234,567.89", "café CAFÉ",
  "İı Iı", "Ωω\u2126", "\ud800x\udc00", "word word", "a-b_c.d", "αβγ ΑΒΓ",
];
const curated = [
  ["\\d+", ""], ["\\d+", "g"], ["\\w+@\\w+\\.\\w+", "g"], ["^\\s+|\\s+$", "g"],
  ["(\\d{4})-(\\d{2})-(\\d{2})", ""], ["(?<y>\\d{4})-(?<m>\\d{2})-(?<d>\\d{2})", ""],
  ["https?://([^/\\s]+)(/[^\\s?#]*)?(\\?[^\\s#]*)?(#\\S*)?", ""], ["\\(?(\\d{3})\\)?[ -]?(\\d{3})-(\\d{4})", "g"],
  ["[A-Z][a-z]+", "g"], ["([a-z])([A-Z])", "g"], ["_(\\w)", "g"], ["<(\\w+)[^>]*>(.*?)</\\1>", ""],
  ["{{\\s*(\\w+)\\s*}}", "g"], ["\\$(\\d+(?:\\.\\d\\d)?)", "g"], ["^.*$", "m"], ["^line", "gm"],
  ["two$", "m"], [".+", "s"], ["\\r?\\n", "g"], ["\\t", "g"], ["\\b\\w", "g"], ["\\B\\w", "g"],
  ["(\\w+)\\s\\1", ""], ["(a+)+b", ""], ["(a|aa)+b", ""], ["(?:ab)+", ""], ["(ab)*?", ""],
  ["(?=\\w*\\d)\\w+", ""], ["\\w+(?!\\d)", ""], ["(?<=\\$)\\d+", "g"], ["(?<!\\$)\\b\\d+", "g"],
  ["(?<=(a)(b))c", ""], ["(?<=\\1(a))b", ""], ["(?<!a)b", "g"], ["\\d{1,3}(?=(\\d{3})+(?!\\d))", "g"],
  ["straße", "i"], ["STRASSE", "i"], ["ß", "iu"], ["\\u212a", "i"], ["\\u212a", "iu"], ["k", "iu"],
  ["s", "iu"], ["[a-z]+", "i"], ["[a-z]+", "iu"], ["\\w+", "iu"], ["\\W+", "iu"], ["[^\\W]+", "iu"],
  ["[\\W\\d]", "iu"], ["\\bs\\b", "iu"], ["ω", "i"], ["Ω", "iu"], ["[α-ω]+", "i"], ["ǆ", "i"], ["ǆ", "iu"],
  ["café", "i"], ["i", "i"], ["I", "iu"], ["\u0130", "iu"], ["\u0131", "i"],
  [".", "u"], [".", "gu"], ["^.$", "u"], ["^.$", ""], ["😀", "g"], ["[😀]", "u"], ["[😀]", ""],
  ["\\u{1F600}", "u"], ["\\ud83d\\ude00", "u"], ["\\ud83d", "u"], ["\\ud83d", ""], ["[\\ud83d\\ude00]", "u"],
  ["\\u{10400}", "iu"], ["[^x]", "u"], ["\\S+", "u"], ["(?:)", "g"], ["", "g"], ["a*", "g"], ["a*?", "gy"],
  ["a|b|c", "y"], ["b", "y"], ["\\0", ""], ["a\\0b", ""], ["[\\0]", ""], ["\\x41", ""], ["\\u0041", ""],
  ["\\cJ", ""], ["[\\b]", ""], ["\\101", ""], ["\\08", ""], ["\\8", ""], ["(a)\\2", ""], ["(a)\\1\\2", ""],
  ["\\1(a)", ""], ["(a)|\\1b", ""], ["{", ""], ["}", ""], ["]", ""], ["a{", ""], ["a{1", ""], ["a{1,", ""],
  ["a{,2}", ""], ["x{2}y", ""], ["\\k", ""], ["\\k<a>", ""], ["(?<a>x)\\k<a>", ""], ["\\k<a>(?<a>x)", ""],
  ["(?<a>x)|(?<a>y)", ""], ["(?:(?<a>x)|(?<a>y))\\k<a>", ""], ["[\\d-x]", ""], ["[x-\\d]", ""], ["[\\c_]", ""],
  ["\\c", ""], ["\\c0", ""], ["[\\c0]", ""], ["\\a", ""], ["\\_", ""], ["\\-", ""], ["\\p{L}", ""],
  ["(?=a)*", ""], ["(?=a){2}", ""], ["(?!a)+b", ""], ["a(?i:B)c", ""], ["(?i:a(?-i:b))", ""], ["(?s:.)a", ""],
  ["(?m:^)b", ""], ["(?i-s:a.)", "s"], ["(?ims:x)", ""], ["(?-ims:x)", "ims"],
  ["[\\w&&\\d]", "v"], ["[\\w--\\d]+", "v"], ["[[a-z]--[aeiou]]+", "v"], ["[\\q{abc|d|}x]", "v"],
  ["[\\q{abc|a}]", "v"], ["[\\q{ABC}]", "vi"], ["[^\\d]", "v"], ["[[^a]&&[a-c]]", "v"], ["[a&&b]", "v"],
  ["[\\q{}]", "v"], ["[a-c--b]", "v"], ["[\\&]", "v"], ["[\\-]", "v"], ["\\w", "v"], ["[a\\--]", "v"],
  ["((a)|b)+", ""], ["(a?)+", ""], ["(a*)*b", ""], ["(?:a|())*", ""], ["(a|b)*c", ""], ["(z)((a+)?(b+)?(c))*", ""],
  ["(.*?)a(?!(a+)b\\2c)\\2(.*)", ""], ["(?=(a+))a*b\\1", ""], ["(?=(a))?", ""], ["(a)?(?:b\\1)", ""],
  ["^(?:a|ab)*c$", ""], ["(x)(?:y(?<z>z))?", ""], ["(?:(a)|b)*", ""], ["(a)|(b)", "g"],
];
for (const [p, f] of curated) {
  const inputs = [];
  for (let i = 0; i < 6; i++) inputs.push(pick(texts));
  run(p, f, inputs);
}

// ---- random patterns ----
const lit = ["a", "b", "c", "A", "B", "x", "ß", "ſ", "K", "😀", "\\n", "-", " ", "1", "9"];
const cls = ["[abc]", "[^ab]", "[a-c]", "[A-Za-z]", "\\d", "\\D", "\\w", "\\W", "\\s", "\\S", ".", "[\\s\\d]", "[^]", "[]", "[\\w-]", "[a-]"];
const asserts = ["^", "$", "\\b", "\\B"];
let groupCount = 0;
// Group names are unique within a random pattern: qjs accepts some duplicates that the spec
// (and djs) reject, see docs/decisions.md.
let nameCount = 0;
function atom(depth) {
  const r = below(10);
  if (depth > 3 || r < 4) return pick(lit);
  if (r < 6) return pick(cls);
  if (r === 6) return pick(asserts);
  if (r === 7) {
    if (groupCount > 0 && below(2)) return "\\" + (1 + below(groupCount));
    if (below(3) === 0) return "\\k<n" + below(nameCount + 1) + ">";
  }
  const kind = below(8);
  const body = alt(depth + 1);
  if (kind < 3) { groupCount++; return "(" + body + ")"; }
  if (kind === 3) return "(?:" + body + ")";
  if (kind === 4) { groupCount++; return "(?<n" + nameCount++ + ">" + body + ")"; }
  return pick(["(?=", "(?!", "(?<=", "(?<!", "(?i:", "(?-i:"]) + body + ")";
}
function quant() {
  const q = pick(["", "", "", "*", "+", "?", "*?", "+?", "??", "{2}", "{1,2}", "{0,}", "{2,}?", "{0}", "{1,3}?"]);
  return q;
}
function seq(depth) {
  let s = "";
  const n = 1 + below(depth > 1 ? 2 : 4);
  for (let i = 0; i < n; i++) s += atom(depth) + quant();
  return s;
}
function alt(depth) {
  let s = seq(depth);
  while (below(4) === 0) s += "|" + seq(depth);
  return s;
}
const alphabet = ["a", "b", "c", "A", "B", "x", " ", "1", "\n", "ß", "ſ", "K", "k", "s", "S", "😀", "\ud83d", "-"];
function randInput() {
  let s = "";
  const n = below(9);
  for (let i = 0; i < n; i++) s += pick(alphabet);
  return s;
}
const flagSets = ["", "", "g", "i", "m", "s", "u", "iu", "y", "gu", "v", "vi", "ims"];
for (let i = 0; i < 3000 * SCALE; i++) {
  groupCount = 0;
  nameCount = 0;
  const p = alt(0);
  const inputs = [];
  for (let k = 0; k < 4; k++) inputs.push(randInput());
  run(p, pick(flagSets), inputs);
}

// ---- random syntax (mostly errors) ----
const synChars = ["(", ")", "[", "]", "{", "}", "?", "*", "+", "|", "\\", "^", "$", ".", "-", "a", "1", ",", "<", ">", "=", "!", ":", "k", "u", "x", "p", "q", "c", "&", "0", "8", "i", "-", "/", "d", "b"];
for (let i = 0; i < 4000 * SCALE; i++) {
  let p = "";
  const n = 1 + below(7);
  for (let k = 0; k < n; k++) p += pick(synChars);
  run(p, pick(["", "u", "v", "i", "iu"]), [pick(["", "a", "a1", "{a}", "-"])]);
}

console.log(lines.join("\n"));
