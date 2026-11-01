// split and replace on unmodified RegExps take leaf fast paths (D1503); any change to the
// protocol (exec, flags, lastIndex, constructor, species, Symbol.match) must still be
// observed exactly as the spec says.
const text = "a1b22c333d,e;f g";
console.log(JSON.stringify(text.split(/\d+/)), JSON.stringify(text.split(/(\d)(\d)?/, 7)), JSON.stringify("abc".split(/(?:)/)));
console.log(JSON.stringify("a😀b".split(/(?:)/u)), JSON.stringify("a😀b".split(/(?:)/)), JSON.stringify("xyx".split(/x/y)), JSON.stringify("  a  b ".split(/\s*/)));
console.log(text.replace(/\d+/g, "<$&>"), text.replace(/(\d)(\d)?/g, "[$2|$1|$3|$01|$`]"), "aaa".replace(/a*?/g, "-"), "😀😀".replace(/(?:)/gu, "."));
console.log("2024-07-04".replace(/(?<y>\d+)-(?<m>\d+)-(?<d>\d+)/g, "$<d>.$<m>.$<y>$<none>|$<y"), "abc".replace(/b/gy, "X"), "bbc".replace(/b/gy, "X"));
const g = /o/g; g.lastIndex = 5;
console.log("foo boo".replace(g, "0"), g.lastIndex, JSON.stringify("a,b".split(g)), g.lastIndex);
const log = [];
const orig = RegExp.prototype.exec;
RegExp.prototype.exec = function (s) { log.push("exec@" + this.lastIndex + (this.sticky ? "y" : "")); return orig.call(this, s); };
console.log(JSON.stringify("a,b,c".split(/,/)), "a.b.c".replace(/\./g, "-"), log.join(" "));
RegExp.prototype.exec = orig;
log.length = 0;
const r = /,/g;
Object.defineProperty(r, "flags", { get() { log.push("flags"); return "g"; } });
console.log("a,b".replace(r, ";"), JSON.stringify("a,b".split(r)), log.join(" "));
const r2 = /x/g;
r2.lastIndex = { valueOf() { log.push("valueOf"); return 0; } };
console.log("axbx".replace(r2, "y"), r2.lastIndex, log.join(" "));
class Sub extends RegExp {}
console.log(JSON.stringify("a-b".split(new Sub("-"))), "a-b".replace(new Sub("-", "g"), "+"));
const r3 = /-/;
r3.constructor = function (src, flags) { log.push("ctor " + flags); return new RegExp(src, flags); };
r3.constructor[Symbol.species] = r3.constructor;
console.log(JSON.stringify("a-b".split(r3)), log.join(" "));
console.log(JSON.stringify("a-b-c".split(/-/, { valueOf() { return 2; } })), JSON.stringify("a-b".split(/-/, 0)), JSON.stringify("".split(/-/)), JSON.stringify("".split(/(?:)/)));
const big = "word ".repeat(2000);
console.log(big.split(/\s+/).length, big.replace(/o/g, "0").slice(0, 20), big.replace(/(w)(o)/g, "$2$1").length);
