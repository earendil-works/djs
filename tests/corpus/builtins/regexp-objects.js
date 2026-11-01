// RegExp objects: construction, flags and source, lastIndex with g/y, exec results
// (index, input, groups, indices), toString, errors with qjs messages.
const r = /a(b)?(?<n>c)/gd;
console.log(String(r), r.source, r.flags, r.global, r.hasIndices, r.sticky, r.lastIndex, Object.prototype.toString.call(r));
let m = r.exec("xxabcac");
console.log(JSON.stringify(m), m.index, m.input, JSON.stringify(m.groups), JSON.stringify(m.indices), JSON.stringify(m.indices.groups), r.lastIndex);
m = r.exec("xxabcac");
console.log(JSON.stringify(m), r.lastIndex, r.exec("xxabcac"), r.lastIndex);
console.log(Object.keys(/x/.exec("x")).join(), Object.getPrototypeOf(/(?<a>x)/.exec("x").groups));
const y = /o/y;
console.log(y.test("foo"), y.lastIndex, (y.lastIndex = 1, y.test("foo")), y.lastIndex, y.test("foo"), y.test("foo"), y.lastIndex);
const g = /o/g;
g.lastIndex = 100;
console.log(g.exec("foo"), g.lastIndex, /o/.exec("foo").index);
console.log(new RegExp("x").test("axb"), RegExp("/", "").source, String(new RegExp("\n")), new RegExp("").source, String(new RegExp()));
console.log(RegExp.prototype.source, RegExp.prototype.global, RegExp.prototype.flags, RegExp.prototype.toString());
console.log(RegExp(r) === r, new RegExp(r) === r, new RegExp(r, "i").flags, new RegExp(r).flags, RegExp.length, RegExp.name);
console.log(/[/]\//.source, /\//.test("/"), String(/[\]/]x/), /\u{1F600}/u.test("😀"), /^.$/u.test("😀"), /^.$/.test("😀"));
console.log(/(?<year>\d{4})-(?<month>\d{2})/.exec("on 2024-07!").groups.month, /(a)|(b)/.exec("b").length, /(a)|(b)/.exec("b")[1]);
console.log(/a.c/s.test("a\nc"), /a.c/.test("a\nc"), /^b/m.test("a\nb"), /ABC/i.test("abc"), /\bfoo\b/.test("a foo b"));
console.log(/(\d+)\s\1/.test("12 12"), /(?<=\$)\d+/.exec("cost $42")[0], /\d+(?!px)/.exec("12px 34em")[0]);
for (const [p, f] of [["(", ""], ["a", "gg"], ["a", "x"], ["[b-a]", ""], ["a**", ""], ["\\", ""], ["(?<n>a)(?<n>b)", ""]] ) {
  try { new RegExp(p, f); console.log("ok", p); } catch (e) { console.log(e.name, e.message); }
}
for (const f of [() => RegExp.prototype.exec.call({}, "x"), () => RegExp.prototype.test.call(1, "x"), () => RegExp.prototype.global,
  () => Object.getOwnPropertyDescriptor(RegExp.prototype, "global").get.call({}), () => RegExp.prototype.toString.call(1),
  () => { const f = Object.freeze(/a/g); f.exec("a"); }, () => /a/[Symbol.match].call(1, "a")]) {
  try { console.log(f()); } catch (e) { console.log(e.name, e.message); }
}
const desc = Object.getOwnPropertyDescriptor(/x/, "lastIndex");
console.log(desc.value, desc.writable, desc.enumerable, desc.configurable, Object.getOwnPropertyNames(/x/).join());
console.log(RegExp[Symbol.species] === RegExp, typeof RegExp.prototype[Symbol.split], RegExp.prototype.exec.name, RegExp.prototype[Symbol.replace].name);
const custom = { exec() { return { 0: "zz", length: 1, index: 0 }; } };
console.log(RegExp.prototype.test.call(custom, "q"), RegExp.prototype[Symbol.match].call({ flags: "", exec: () => null }, "q"));
console.log(RegExp.escape("a.b*c?(1)[2]{3}|^$\\/ -=,"), RegExp.escape("1x"), RegExp.escape("\t\n é\u2028"));
let fresh = []; for (let i = 0; i < 2; i++) fresh.push(/x/g); console.log(fresh[0] === fresh[1]);
