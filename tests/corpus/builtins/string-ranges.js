// Substrings and joins of ropes, slices and append buffers, one-byte and two-byte (0269:
// the result is written straight from the leaves; a two-byte source narrows when the
// range has no unit above 0xFF).
const wide = "ŝ";
const parts = [];
for (let i = 0; i < 40; i++) parts.push("line " + i + (i % 7 === 3 ? " " + wide : "") + "\n");
const rope = parts[0] + parts[1] + parts[2] + parts[3] + parts[4] + parts[5];
console.log(rope.length, JSON.stringify(rope.slice(3, 30)), JSON.stringify(rope.slice(0, 20)));
console.log(JSON.stringify(rope.slice(24, 40)), rope.slice(24, 40).length);
let out = "";
for (const p of parts) out += p;
console.log(out.length, JSON.stringify(out.slice(100, 140)), JSON.stringify(out.slice(5, 9)));
const narrow = out.slice(0, 30);
console.log(narrow === parts.slice(0, 3).join("").slice(0, 30), narrow.length);
const big = "x".repeat(300) + "\n" + "y".repeat(200);
const line = big.split("\n")[1];
const mixed = ["a", line.slice(10, 20), rope, wide + "b", out.slice(30, 60)];
const joined = mixed.join("|");
console.log(joined.length, JSON.stringify(joined.slice(0, 40)), JSON.stringify(joined.slice(-30)));
const tmpl = `=== ${"head"} ${parts.length}\n${out}`;
console.log([tmpl, tmpl].join("\n").length, JSON.stringify([tmpl, rope].join("\n").slice(10, 50)));
console.log(JSON.stringify((rope + out).substring(60, 20)), (rope + out).substring(0, 3));
console.log(JSON.stringify(("é" + "ab".repeat(20) + wide).slice(1, 10)));
