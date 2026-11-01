// RegExp (0033): literals, exec results, lastIndex, the String methods and replacement
// patterns, matchAll, split with captures, sticky and unicode matching, and a match long
// enough to be suspended many times in the suspension mode.
const date = /(?<y>\d{4})-(?<m>\d{2})-(?<d>\d{2})/u;
const m = date.exec("released 2024-07-04.");
console.log(m[0], m.index, m.groups.y, m.groups.m, m.groups.d, m.length);
console.log("2024-07-04".replace(date, "$<d>/$<m>/$<y>"), "a-b-c".split(/-/), "a1b2".replace(/\d/g, (d) => d * 2));
const words = "the quick brown fox jumps";
console.log(words.match(/\b\w/g).join(""), words.search(/fox/), [...words.matchAll(/o(\w)/g)].map((x) => x[1] + x.index).join());
const g = /\w+/g;
let w, seen = [];
while ((w = g.exec("ab cd ef")) !== null) seen.push(w[0] + "@" + g.lastIndex);
console.log(seen.join(" "), g.lastIndex);
console.log(/^[\u{1F600}-\u{1F64F}]$/u.test("😃"), "😀a😀".split(/(?:)/u).length, /(?<!\$)\b\d+/.exec("$5 10")[0]);
const long = "ab".repeat(60) + "c";
console.log(/^(?:a|b)*c$/.test(long), /(a|b)*d/.test(long), long.replace(/ab/g, "").length);
console.log(String(/x\/y/gi), new RegExp("a/b").source, /[]/.test("a"), /[^]/.test("\n"));
try { new RegExp("a)"); } catch (e) { console.log(e.name, e.message); }
const rs = [];
for (let i = 0; i < 3; i++) rs.push(/z/g);
console.log(rs[0] === rs[1], rs[1].lastIndex);
