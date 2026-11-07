// The own properties of the builtin prototypes in qjs's order (0111, D2150): methods
// in qjs's order and `constructor` where qjs puts it. Keys djs does not have are left
// out on both sides: Annex B's HTML string methods, setYear, compile and the
// __defineGetter__ family, and qjs's own Function.prototype.fileName/lineNumber/
// columnNumber.
const absent = new Set(["anchor", "big", "blink", "bold", "fixed", "fontcolor", "fontsize", "italics",
  "link", "small", "strike", "sub", "sup", "setYear", "compile", "__defineGetter__",
  "__defineSetter__", "__lookupGetter__", "__lookupSetter__", "fileName", "lineNumber", "columnNumber"]);
const P = Object.getPrototypeOf;
const list = [];
for (const n of Object.getOwnPropertyNames(globalThis).sort()) {
  const v = globalThis[n];
  // djs has no SharedArrayBuffer, qjs no TextEncoder or TextDecoder.
  if (n !== "SharedArrayBuffer" && n !== "TextEncoder" && n !== "TextDecoder" && typeof v === "function" && /^[A-Z]/.test(n) && v.prototype) {
    list.push([n + ".prototype", v.prototype]);
  }
}
list.push(
  ["%IteratorPrototype%", P(P([][Symbol.iterator]()))],
  ["%ArrayIteratorPrototype%", P([][Symbol.iterator]())],
  ["%StringIteratorPrototype%", P(""[Symbol.iterator]())],
  ["%MapIteratorPrototype%", P(new Map().entries())],
  ["%SetIteratorPrototype%", P(new Set().values())],
  ["%RegExpStringIteratorPrototype%", P(/a/[Symbol.matchAll](""))],
  ["%GeneratorFunction.prototype%", P(function* () {})],
  ["%GeneratorPrototype%", P(function* () {}).prototype],
  ["%AsyncFunction.prototype%", P(async function () {})],
  ["%AsyncGeneratorFunction.prototype%", P(async function* () {})],
  ["%AsyncGeneratorPrototype%", P(async function* () {}).prototype],
  ["%AsyncIteratorPrototype%", P(P(async function* () {}).prototype)],
  ["%TypedArray.prototype%", P(Int8Array.prototype)],
  ["%IteratorHelperPrototype%", P([].values().map((x) => x))],
  ["%WrapForValidIteratorPrototype%", P(Iterator.from({ next() {} }))],
);
for (const [name, o] of list) {
  console.log(name, Reflect.ownKeys(o).map(String).filter((k) => !absent.has(k)).join());
}
