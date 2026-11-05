// The RegExp compile cache (D3138), shared programs (D3136) and RegExpExecOwn (D3137) must
// be unobservable: each RegExp is its own object with its own lastIndex, and changing exec,
// lastIndex or the flag getters is still seen exactly as the spec says.
const made = [];
for (let i = 0; i < 20; i++) made.push(new RegExp("a(b)?|c" + (i % 10), i % 3 ? "g" : "gi"));
const twins = [new RegExp("x", "g"), new RegExp("x", "g"), RegExp("x", "g")];
twins[0].lastIndex = 2;
console.log(made.length, made[0] === made[10], twins[0] === twins[1], twins.map((r) => r.lastIndex).join(), twins[1].test("axbx"), twins[1].lastIndex, twins[2].lastIndex);
console.log(made.map((r) => r.flags + ":" + r.source).slice(0, 4).join(" "), "AB".replace(made[0], "-"), "AB".replace(made[1], "-"));
try { new RegExp("(", "g"); } catch (e) { console.log(e.name); }
try { new RegExp("(", "g"); } catch (e) { console.log(e.name, "again"); }
try { new RegExp("x", "gg"); } catch (e) { console.log(e.name, "flags"); }
const re = /o+/gi;
const copies = [new RegExp(re), new RegExp(re, "g"), new RegExp(re, "ig"), new RegExp(re, "y")];
console.log(copies.map((r) => r.flags + "|" + r.source + "|" + r.lastIndex).join(" "), copies.map((r) => "fOO boo".replace(r, "_")).join(" "));
const long = "z".repeat(300) + "|q";
console.log(new RegExp(long).test("q"), new RegExp(long).source.length);
// matchAll copies lastIndex and flags; the iterator's regexp is a new object.
const ma = /(\w)(\d)/g;
ma.lastIndex = 3;
console.log(JSON.stringify([..."a1b2c3d4".matchAll(ma)].map((m) => m[0] + "@" + m.index)), ma.lastIndex);
console.log(JSON.stringify([..."x\nexport y(a)\nexport z(b)".matchAll(/^export (\w)\((\w)\)/gm)].map((m) => m.slice(1).join())));
const fl = /q/g;
const log = [];
Object.defineProperty(fl, "global", { get() { log.push("global"); return true; } });
console.log(JSON.stringify([..."qq".matchAll(fl)].map((m) => m.index)), fl.flags, log.join());
// exec and test with an own exec, a changed prototype exec, a lastIndex object.
const own = /b/g;
own.exec = function (s) { log.push("own exec"); return null; };
console.log(own.test("abc"), "abc".replace(own, "B"), log.join());
const orig = RegExp.prototype.exec;
RegExp.prototype.exec = function (s) { log.push("proto exec " + this.source); return orig.call(this, s); };
console.log(/c/.test("abc"), JSON.stringify("abc".match(/c/)), [..."cc".matchAll(/c/g)].length, log.slice(-3).join());
RegExp.prototype.exec = orig;
const li = /d/g;
li.lastIndex = { valueOf() { log.push("valueOf"); return 1; } };
console.log(li.test("dd"), li.lastIndex, JSON.stringify(li.exec("ddd")), log.slice(-1).join());
const frozen = Object.freeze(/e/g);
try { frozen.exec("e"); } catch (e) { console.log(e.name, "frozen exec"); }
try { frozen.test("x"); } catch (e) { console.log(e.name, "frozen test"); }
console.log(/^b/.exec("ab"), /^b/m.exec("a\nb").index, /^b/y.test("ab"), JSON.stringify("a\r\nb\u2028c".match(/^\w/gm)));
console.log("x  y\t\nz".replace(/\s+/g, " "), "éé  ü".replace(/\s+/g, "_"), "日本  語".replace(/\s+/g, "$&|"), "a b".replace(/\s+/g, "日"));
