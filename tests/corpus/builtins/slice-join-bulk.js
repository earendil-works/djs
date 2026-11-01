// slice's and join's one-step paths (0129): a plain dense slice copied at once, a join of
// flat one-byte strings written straight from the cells, and every case that must fall
// back (holes, species, ropes, two-byte strings, numbers, prototype elements); and split.
const a = ["alpha", "beta", "gamma", "delta", "epsilon"];
console.log(JSON.stringify([a.slice(), a.slice(1, 3), a.slice(-2), a.slice(3, 1), a.slice(9)]));
const s = a.slice(1);
s.push("z");
console.log(a.length, s.length, s.join("|"));
const holes = [1, , 3];
const hs = holes.slice(0, 3);
console.log(hs.length, 1 in hs, JSON.stringify(hs));
Array.prototype[1] = "P";
console.log(holes.slice(0, 3).join("-"), holes.join("-"));
delete Array.prototype[1];
class Sub extends Array {}
const sub = Sub.from(a).slice(1, 3);
console.log(sub instanceof Sub, sub.join());
const own = a.slice();
own.constructor = { [Symbol.species]: function (n) { return { length: n, tag: "species" }; } };
const r = own.slice(0, 2);
console.log(r.tag, r[0], r[1], r.length);
console.log([].join("x") === "", [null, undefined, "x"].join("+"), ["a", "b"].join(""));
let rope = "";
for (let i = 0; i < 30; i++) rope += "r" + i;
console.log(["<", rope, ">"].join(" ").length, ["é", "ü"].join("ß"), ["€", "x"].join("-"));
console.log([1, "a", 2.5, true].join("/"), ["x", "y"].join("€"), ["x", { toString() { return "o"; } }].join());
const big = Array.from({ length: 50 }, (_, i) => "line " + i);
const j = big.slice(10, 40).join("\n");
console.log(j.length, j.split("\n").length, j.slice(0, 20));
// split's pieces copied straight from the string, empty ones shared.
console.log(JSON.stringify([",a,,b,".split(","), "abc".split("abc"), "x".split("y"), "a,b,c".split(",", 1), "a,b".split(",", 0), "a,b,c".split(",", 2)]));
const sp = rope.split("r1");
console.log(sp.length, sp[0], sp[1], sp[sp.length - 1], "".split(",").length, "".split("").length);
const same = "whole";
console.log(same.split("-")[0] === same, "é,ü".split(","), "a€b".split("€"));
