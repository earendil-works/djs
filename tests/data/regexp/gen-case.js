// Generates case.txt from QuickJS: full case mappings and regexp case-insensitive
// equivalences for every code point that has a case mapping. Run ./gen-case.sh.
//
//   map  <c>  <toUpperCase>  <toLowerCase>     code points in hex, space separated lists
//                                              joined by ","; only for c with a mapping
//   eq   <c>  <x>  <i>  <ui>                   /^c$/i.test(x) and /^c$/ui.test(x) as 0/1
//                                              (i is "-" when c or x is not in the BMP)

const hex = (s) => Array.from(s, (ch) => ch.codePointAt(0).toString(16)).join(",");
const esc = (c) => "\\u{" + c.toString(16) + "}";
const lines = [];
for (let c = 0; c <= 0x10ffff; c++) {
  if (c >= 0xd800 && c <= 0xdfff) continue;
  const s = String.fromCodePoint(c);
  const u = s.toUpperCase(), l = s.toLowerCase();
  if (u === s && l === s) continue;
  lines.push(["map", c.toString(16), hex(u), hex(l)].join("\t"));
  // Candidates: single code point results of case mapping, two levels deep.
  const cand = new Set();
  const add = (t) => { if ([...t].length === 1) cand.add(t.codePointAt(0)); };
  for (const t of [u, l]) {
    add(t);
    if ([...t].length === 1) { add(t.toUpperCase()); add(t.toLowerCase()); }
  }
  cand.delete(c);
  const reU = new RegExp("^" + esc(c) + "$", "ui");
  const reI = c <= 0xffff ? new RegExp("^" + s + "$", "i") : null;
  for (const x of [...cand].sort((a, b) => a - b)) {
    const xs = String.fromCodePoint(x);
    const i = reI && x <= 0xffff ? (reI.test(xs) ? "1" : "0") : "-";
    lines.push(["eq", c.toString(16), x.toString(16), i, reU.test(xs) ? "1" : "0"].join("\t"));
  }
}
console.log(lines.join("\n"));
