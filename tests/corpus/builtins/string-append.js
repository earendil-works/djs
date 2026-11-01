// `s += x` builds strings in a buffer in place (D1610): values must stay immutable when an
// older, shorter string over the same buffer is extended, read, or flattened.
// difftest: no-stress 5 s suspended, 14 s under GC stress (debug, 0053)
let s = "start:";
const kept = [];
for (let i = 0; i < 3000; i++) {
  s += String.fromCharCode(97 + (i % 26));
  if (i % 997 === 0) kept.push(s);
}
console.log(s.length, s.slice(0, 12), s.slice(-5), kept.map((k) => k.length).join());
const fork = kept[1] + "FORK";
const fork2 = kept[1] + "OTHER";
console.log(fork.slice(-8), fork2.slice(-8), kept[1].length, s.length, s.indexOf("FORK"), s.lastIndexOf("zab"));
let t = s;
t += "\u2603 snow";
t += "!";
console.log(t.length, t.slice(-8), s.length, s.slice(-3), t.charCodeAt(s.length), JSON.stringify(t.slice(s.length - 2)));
let u = "";
for (let i = 0; i < 500; i++) { u += "row " + i + "\n"; if (i % 100 === 0) u.toUpperCase(); }
console.log(u.length, u.split("\n").length, u.slice(0, 12).replace(/\n/g, "|"), u === u.slice(0), u.endsWith("499\n"));
let w = "x".repeat(300);
const map = new Map();
for (let i = 0; i < 50; i++) { w += i; map.set(w, i); }
console.log(map.get(w), map.size, [...map.keys()][10].length, map.has(w.slice(0, 310)), w.length);
const parts = [];
let acc = "head ".repeat(60);
for (let i = 0; i < 40; i++) { acc += `<${i}>`; parts.push(acc); }
console.log(parts[5].length, parts[39].length, parts[5].slice(-4), parts[39].slice(-4), parts[5] === parts[39].slice(0, parts[5].length));
