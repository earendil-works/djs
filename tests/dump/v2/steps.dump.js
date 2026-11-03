// djs-dump 2: 3 bindings (sleep, step, main).
// at idle point 1: written by the exporter (0172), kept as a golden fixture.
// Load it with Vm::load_dump (djs REPL: .restore FILE), which passes in $dump.
// Builtins are referenced by name, never written. The host globals later, print are re-bound by name.
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
$dump.p1 = $dump.pending(1, "later", [10]);
const sleep = (ms) => later(ms);
async function step(label, ms) {
  await sleep(ms);
  print("done", label);
  return label + ":" + ms;
}
async function main() {
  const results = [];
  for (const label of ["a", "b", "c"]) {
    results.push(await step(label, 10));
  }
  print(JSON.stringify(results));
}
$dump.a2 = $dump.resume(step, { at: "await#1", awaiting: $dump.p1, args: ["a", 10], bindings: { label: "a", ms: 10 } });
$dump.n3 = [];
$dump.i4 = $dump.advance(Array.prototype.values.call(["a", "b", "c"]), 1);
$dump.a5 = $dump.resume(main, { at: "await#1", awaiting: $dump.a2, bindings: { results: $dump.n3, label: "a" }, operands: [["call.callee", Array.prototype.push], ["call.this", $dump.n3]], loops: [{ kind: "for-of", iterator: $dump.i4, next: $dump.builtin("%ArrayIteratorPrototype%.next") }] });
