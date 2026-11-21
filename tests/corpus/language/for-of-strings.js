// for-of over strings and typed arrays through the declined iter_next_loc (0288): ASCII,
// Latin-1, two-byte and astral code points, the last step, a loop that breaks out.
const out = [];
for (const s of ["abc", "äbç", "a€b", "x😀y", "", "😀"]) { const r = []; for (const c of s) r.push(c); out.push(r.join("|"), r.length); }
const ta = new Uint8Array([7, 8, 9]); let t = 0; for (const x of ta) t += x; out.push(t);
let k = 0; for (const c of "abcdef") { if (c === "d") break; k++; } out.push(k);
const it = "hey"[Symbol.iterator](); for (const c of it) out.push(c); out.push(JSON.stringify(it.next()));
const big = "ab".repeat(500); let n = 0; for (const c of big) n += c === "a" ? 1 : 0; out.push(n);
for (const [i, c] of [..."xy"].entries()) out.push(i + c);
console.log(out.join(","));
