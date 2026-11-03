"djs-dump 2";
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
$dump.p1 = $dump.pending(4, "later", ["a2!"]);
var agen = async function* agen() {
  yield await later("a1");
  yield "a2";
};
async function main() {
  const out = [];
  for await (const x of ["s1", later("s2")]) out.push(x);
  for await (const x of agen()) out.push(await later(x + "!"));
  print(out.join());
}
$dump.n2 = ["s1", "s2", "a1!"];
$dump.a3 = $dump.resume(agen, { at: "yield#2" });
$dump.a4 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, bindings: { out: $dump.n2, x: "a2" }, operands: [["call.callee", Array.prototype.push], ["call.this", $dump.n2]], loops: [{ kind: "for-await", iterator: $dump.a3, next: $dump.builtin("%AsyncGeneratorPrototype%.next") }] });
