// One-unit strings (0287, D3560): s[i], charAt, at, slices, split(""), iteration and
// numbers, ASCII and not, across collections.
const s = "héllo wörld €x";
const out = [];
out.push(s[0], s[1], s[13], s.charAt(4), s.at(-1), s.slice(2, 3), s.substring(12, 13), [...s].join("|"), "abc".split("").join("-"));
out.push(String(0), String(7), String(10), String(-3), 5 + "", `${9}`, (3).toString(), String(9.5));
const r = "ab".repeat(3) + "c"; out.push(r[6], r[0]);
const big = "x".repeat(20) + "y"; out.push(big.slice(20, 21));
for (const c of "a\u{1F600}b") out.push(c.length);
const m = new Map(); for (const c of "hello") m.set(c, (m.get(c) || 0) + 1); out.push([...m].join(";"));
out.push("a" === "abc"[0], "abc"[1] < "abc"[2]);
console.log(out.join(","));
const keep = []; for (let k = 0; k < 2000; k++) keep.push("xyz"[k % 3] + k); console.log(keep.length, keep[1999], "q"[0] === "q");
