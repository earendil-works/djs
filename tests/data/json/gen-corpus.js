// Generates corpus.txt: JSON.parse and JSON.stringify results as computed by QuickJS.
// Run ./gen-corpus.sh (uses ~/Development/dquickts/qjs).
//
// One case per line, tab separated. Strings are JSON string literals (JSON.stringify).
//   parse      <input>  ok   <dump>
//   parse      <input>  err  <message>  <line>:<col>
//   stringify  <input JSON text>  <space>  <output>
// <dump> is a canonical form of the parsed value: null, true, false, n:<String(x)> (n:-0
// for negative zero), s:<string literal>, [a,b], {"k":v} in property order.
// <space> is "-" (none), n:<number> or s:<string literal>.

function dump(v) {
  if (v === null) return "null";
  if (typeof v === "boolean") return String(v);
  if (typeof v === "number") return "n:" + (Object.is(v, -0) ? "-0" : String(v));
  if (typeof v === "string") return "s:" + JSON.stringify(v);
  if (Array.isArray(v)) return "[" + v.map(dump).join(",") + "]";
  return "{" + Object.keys(v).map((k) => JSON.stringify(k) + ":" + dump(v[k])).join(",") + "}";
}

const out = [];
function parseCase(input) {
  if (input.includes("\t") && false) return;
  try {
    const v = JSON.parse(input);
    out.push(["parse", JSON.stringify(input), "ok", dump(v)].join("\t"));
  } catch (e) {
    const m = /<input>:(\d+):(\d+)/.exec(e.stack);
    out.push(["parse", JSON.stringify(input), "err", e.message, m[1] + ":" + m[2]].join("\t"));
  }
}
function stringifyCase(text, space) {
  const v = JSON.parse(text);
  const sp = space === undefined ? "-" : typeof space === "number" ? "n:" + space : "s:" + JSON.stringify(space);
  out.push(["stringify", JSON.stringify(text), sp, JSON.stringify(JSON.stringify(v, null, space))].join("\t"));
}

// Seeded PRNG (xorshift32), so the corpus is reproducible.
let seed = 0x2545f491;
function rnd() {
  seed ^= seed << 13; seed >>>= 0;
  seed ^= seed >>> 17;
  seed ^= seed << 5; seed >>>= 0;
  return seed / 4294967296;
}
function below(n) { return Math.floor(rnd() * n); }
function pick(a) { return a[below(a.length)]; }

// ---------------------------------------------------------------- hand-written cases
const hand = [
  "", " ", "{", "[", "]", "}", ":", ",", "[1,]", '{"a":1,}', "{a:1}", "'x'", '"abc', '"\\x"', '"\\u12"',
  '"\\u12g4"', '"\\', '"\\u', "tru", "nul", "fals", "truex", "true1", "nulls", "01", "00", "-01",
  "1.", "1.e1", "1e", "1e+", "1E-", "-", "--1", "+1", ".5", "-.5", "[1 2]", '{"a" 1}', '{"a":}',
  "1 2", '"a\nb"', '"\t"', "NaN", "Infinity", "-Infinity", '{"a":1}}', "[[[", '"\\uD800"',
  '"\\udc00\\ud800"', "1e400", "-1e400", "1e-400", "-0", "-0.0", "0e0", '{"__proto__":1}',
  "\u00a01", "[1,2", '{"a":1', '{"a"', "{1:2}", "undefined", '"\u0000"', "[,1]", "true false",
  "/*c*/1", "//c\n1", '{"a":1,"a":2}', '{"b":1,"a":2,"b":3}', "[01]", "0x10", "1E+2", "-1.5e-3",
  "\u2028 1", "\ufeff1", "1\u0000", "[1,\u0000]", "\u0000", '"\u007f\u0080\u00ff"',
  '"\u20ac\ud83d\ude00"', '"\ud800"', '"x\udc00y"', "[1,\n  2,]", '{"a":\n tru}', "\n\n  @",
  '"\\q"', "\r\n[\r\n1\r\n,\r\n]", "[-]", "[-a]", "-x", "-\u00e9", "-\u20ac", "[1e]", "[1.]", "@",
  "#", "\\", "[\"a\" \"b\"]", '{"a":1 "b":2}', '{"a":1,"b"}', '{"a":1,"b":}', '{,}', "[,]",
  '{"a":[}', '{"a":{]}', "[{]", "[{}", '"\\/\\b\\f\\n\\r\\t\\"\\\\"', '"\\u0000\\u001F\\u0020"',
  '"\\uDBFF\\uDFFF"', '"\\uD83D"x', '{"":""}', '{"0":1,"1":2,"-1":3,"01":4,"4294967294":5,"4294967295":6}',
  '{"2":1,"10":2,"1":3,"a":4,"b":5}', "[[[[[[[[[[1]]]]]]]]]]", "123456789012345678901234567890",
  "0.1", "0.30000000000000004", "1.7976931348623157e308", "1.7976931348623159e308",
  "5e-324", "2.4703282292062327e-324", "2.4703282292062328e-324", "9007199254740993",
  "-9007199254740993", "1.00000000000000011102230246251565404236316680908203125",
  "12345678901234567890e-10", "1" + "0".repeat(400), "0." + "0".repeat(400) + "1",
  "\t\n\r [ \t\n\r 1 \t\n\r ] \t\n\r", "\u000b1", "\u000c1", '"\u00e9\\u00e9"', "ü", '"ü',
];
for (const h of hand) parseCase(h);

// ---------------------------------------------------------------- numbers
for (let i = 0; i < 300; i++) {
  let s = pick(["", "-"]);
  const intLen = 1 + below(20);
  s += below(10) === 0 ? "0" : String(1 + below(9)) + Array.from({ length: intLen - 1 }, () => below(10)).join("");
  if (below(2)) s += "." + Array.from({ length: 1 + below(20) }, () => below(10)).join("");
  if (below(2)) s += pick(["e", "E"]) + pick(["", "+", "-"]) + below(400);
  parseCase(s);
}

// ---------------------------------------------------------------- random documents
function randString() {
  let s = "";
  const n = below(10);
  for (let i = 0; i < n; i++) {
    const r = below(10);
    if (r < 5) s += String.fromCharCode(97 + below(26));
    else if (r === 5) s += pick(["\\n", "\\t", '\\"', "\\\\", "\\/", "\\b", "\\f", "\\r"]);
    else if (r === 6) s += "\\u" + below(65536).toString(16).padStart(4, "0");
    else if (r === 7) s += pick(["é", "€", "😀", "ß", "\u00ff"]);
    else if (r === 8) s += " ";
    else s += String(below(10));
  }
  return '"' + s + '"';
}
function ws() { return below(4) ? "" : pick([" ", "\n", "\t", "\r\n", "  "]); }
function randJson(depth) {
  const r = depth > 3 ? below(4) : below(7);
  if (r === 0) return pick(["null", "true", "false"]);
  if (r === 1) return String(pick([0, -1, 42, 3.14, -2.5e-8, 1e21, 123456789, 0.1]));
  if (r === 2 || r === 3) return randString();
  if (r === 4 || r === 5) {
    const items = Array.from({ length: below(5) }, () => ws() + randJson(depth + 1) + ws());
    return "[" + items.join(",") + "]";
  }
  const n = below(5);
  const items = [];
  for (let i = 0; i < n; i++) {
    const key = pick(['"a"', '"b"', '"name"', '"0"', '"1"', '"10"', '"é"', '"k' + below(5) + '"', '""', randString()]);
    items.push(ws() + key + ws() + ":" + ws() + randJson(depth + 1) + ws());
  }
  return "{" + items.join(",") + "}";
}
const docs = [];
for (let i = 0; i < 300; i++) {
  const d = ws() + randJson(0) + ws();
  docs.push(d);
  parseCase(d);
}

// ---------------------------------------------------------------- mutated documents (errors)
const junk = ["", "{", "}", "[", "]", ",", ":", '"', "\\", "x", "1", "-", ".", "e", " ", "\n", "t", "\u0000", "é", "€"];
for (let i = 0; i < 600; i++) {
  const d = pick(docs);
  const p = below(d.length + 1);
  const del = below(3);
  parseCase(d.slice(0, p) + pick(junk) + d.slice(p + del));
}

// ---------------------------------------------------------------- stringify
const spaces = [undefined, 2, 4, 0, -3, 12, 1.9, "", "\t", "--", "abcdefghijklmno", "€", " é "];
for (const t of ['null', 'true', '1', '-0', '"x"', '[]', '{}', '[[]]', '{"a":{}}', '[1,[2,[3]]]',
    '{"a":[1,{"b":2}],"c":"x"}', '{"1":1,"b":2,"0":0}', '"\\u2028\\u2029\\ud800\\udc00\\udc00\\ud800 \\u007f\\u0000\\u001f\\b\\f\\n\\r\\t\\"\\\\/"',
    '[1e21,1e-7,123e-20,-1.5,0.1,5e-324,1.7976931348623157e308]']) {
  for (const sp of spaces) stringifyCase(t, sp);
}
for (let i = 0; i < 200; i++) stringifyCase(pick(docs), pick(spaces));

console.log(out.join("\n"));
