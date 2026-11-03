// djs-dump 1: 12 bindings (counter, bytes, small, key, index, graph, frozen, tag, re, iterProto, legacy, makeCounter).
// Run it as a Script to recreate them (djs REPL: .restore FILE). Builtins are
// referenced by name, never written.
globalThis.$dump = {};
$dump.bytes = (s) => {
  const b = atob(s), u = new Uint8Array(b.length);
  for (let i = 0; i < b.length; i++) u[i] = b.charCodeAt(i);
  return u.buffer;
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
// inc, get sharing n
$dump.s1 = (() => {
  let n = 41;
  return { inc: () => ++n, get: () => n };
})();
const counter = { inc: $dump.s1.inc, get: $dump.s1.get };
const bytes = new Uint8Array($dump.bytes("AAMGCQwPEhUYGx4hJCcqLTAzNjk8P0JFSEtOUVRXWl1gY2ZpbG9ydXh7foGEh4qNkJOWmZyfoqWoq66xtLe6vcDDxsnMz9LV2Nve4eTn6u3w8/b5/P8CBQgLDhEUFxodICMmKQ=="));
const small = new Int16Array([-1, 2, 300]);
const key = { k: 1 };
const index = new Map([[key, "object key"], ["when", new Date(1700000000000)]]);
$dump.n3 = { parent: undefined };
$dump.n2 = [$dump.n3];
const graph = { name: "root", children: $dump.n2 };
const frozen = { list: [1, 2] };
const tag = Symbol.for("app.tag");
const re = /ab+c/gi;
const iterProto = $dump.builtin("%ArrayIteratorPrototype%");
var legacy = { big: 12345678901234567890n, [tag]: "tagged" };
function makeCounter(start) { let n = start; return { inc: () => ++n, get: () => n }; }
// Builtins the session changed.
Array.prototype.last = function () { return this[this.length - 1]; };
// Links that close cycles, and attributes.
$dump.n3.parent = graph;
Object.freeze(frozen);
void delete globalThis.$dump;
