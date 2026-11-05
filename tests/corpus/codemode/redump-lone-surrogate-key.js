// Keys holding a lone surrogate ("\ud800" + "9"): a dump quotes a key from the string
// itself, not from its text, where it was U+FFFD (0215, plain-dump fuzzer seed 82489).
const o = { a: 0 };
o["\ud800" + "9"] = 2;
o["\udc00"] = { deep: "\udbff" };
const s = Symbol.for("\ud800x");
const m = new Map([["\udc01", 1]]);
await tools.echo(1);
console.log(JSON.stringify(o), Object.keys(o).map((k) => k.charCodeAt(0)).join());
console.log(Symbol.keyFor(s).charCodeAt(0), s === Symbol.for("\ud800x"), [...m.keys()][0].charCodeAt(0));
