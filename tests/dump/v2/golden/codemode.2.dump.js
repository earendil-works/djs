"djs-dump 2";
$dump.advance = (it, n) => {
  for (let i = 0; i < n; i++) it.next();
  return it;
};
$dump.builtin = (path) => {
  const roots = {
    IteratorPrototype: () => Object.getPrototypeOf(Object.getPrototypeOf([][Symbol.iterator]())),
    ArrayIteratorPrototype: () => Object.getPrototypeOf([][Symbol.iterator]()),
    StringIteratorPrototype: () => Object.getPrototypeOf(''[Symbol.iterator]()),
    MapIteratorPrototype: () => Object.getPrototypeOf(new Map().entries()),
    SetIteratorPrototype: () => Object.getPrototypeOf(new Set().values()),
    RegExpStringIteratorPrototype: () => Object.getPrototypeOf(/./[Symbol.matchAll]('')),
    GeneratorFunction: () => (function* () {}).constructor,
    GeneratorPrototype: () => Object.getPrototypeOf(function* () {}).prototype,
    AsyncFunction: () => (async function () {}).constructor,
    AsyncGeneratorFunction: () => (async function* () {}).constructor,
    AsyncGeneratorPrototype: () => Object.getPrototypeOf(async function* () {}).prototype,
    TypedArray: () => Object.getPrototypeOf(Int8Array),
    TypedArrayPrototype: () => Object.getPrototypeOf(Int8Array.prototype),
    ThrowTypeError: () => Object.getOwnPropertyDescriptor(Function.prototype, 'caller').get,
  };
  const [, acc, root, rest] = /^(get |set )?(%\w+%|[\w$]+)(.*)$/.exec(path);
  let v = root[0] === "%" ? roots[root.slice(1, -1)]() : globalThis[root];
  const segs = rest.match(/\.\[\[Prototype\]\]|\.[\w$]+|\["(?:[^"\\]|\\.)*"\]|\[Symbol\.\w+\]/g) || [];
  segs.forEach((s, i) => {
    if (s === ".[[Prototype]]") return void (v = Object.getPrototypeOf(v));
    const key = s[0] === "." ? s.slice(1) : s.startsWith("[Symbol.") ? Symbol[s.slice(8, -1)] : JSON.parse(s.slice(1, -1));
    if (acc && i === segs.length - 1) {
      const d = Object.getOwnPropertyDescriptor(v, key);
      v = acc === "get " ? d.get : d.set;
    } else v = v[key];
  });
  return v;
};
$dump.p1 = $dump.pending(3, "later", ["aa"]);
$dump.i2 = $dump.advance(Array.prototype.values.call(["a", "b"]), 1);
$dump.a3 = $dump.resume(async function (tools, console) {
// A codemode body (pi's Sandbox): tools and console are its parameters.
console.log("start");
let n = 0;
for (const f of await tools.list()) {
  const body = await tools.read(f);
  n += body.length;
  console.log(f, body);
}
const total = () => n;
await later(0);
console.log("total", total());
return n;

}, { at: "await#2", awaiting: $dump.p1, this: globalThis, args: [tools, console], bindings: { tools: tools, console: console, n: 0, f: "a", body: undefined, total: undefined }, loops: [{ kind: "for-of", iterator: $dump.i2, next: $dump.builtin("%ArrayIteratorPrototype%.next") }] });
$dump.handle("run", $dump.a3);
