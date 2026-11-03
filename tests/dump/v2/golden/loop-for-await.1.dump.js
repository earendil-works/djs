"djs-dump 2";
$dump.advance = (it, n) => {
  for (let i = 0; i < n; i++) it.next();
  return it;
};
$dump.closeIt = (it) => {
  try {
    const m = it.return;
    if (m !== undefined && m !== null) m.call(it);
  } catch {}
};
$dump.closeThrow = (it) => (e) => {
  $dump.closeIt(it);
  throw e;
};
$dump.fromSync = (it, next) => {
  const step = (r, close) => {
    if (Object(r) !== r) return Promise.reject(new TypeError("iterator must return an object"));
    let done, wrapper;
    try {
      done = !!r.done;
      const value = r.value;
      try {
        wrapper = Promise.resolve(value);
      } catch (e) {
        if (!done && close) $dump.closeIt(it);
        throw e;
      }
    } catch (e) {
      return Promise.reject(e);
    }
    return wrapper.then((v) => ({ value: v, done }), !done && close ? $dump.closeThrow(it) : undefined);
  };
  const call = (f, args, close) => {
    let r;
    try {
      r = f.apply(it, args);
    } catch (e) {
      return Promise.reject(e);
    }
    return step(r, close);
  };
  return {
    next(...args) {
      return call(next, args, true);
    },
    return(...args) {
      let m;
      try {
        m = it.return;
      } catch (e) {
        return Promise.reject(e);
      }
      if (m === undefined || m === null) return Promise.resolve({ value: args[0], done: true });
      return call(m, args, false);
    },
    throw(...args) {
      let m;
      try {
        m = it.throw;
      } catch (e) {
        return Promise.reject(e);
      }
      if (m === undefined || m === null) {
        $dump.closeIt(it);
        return Promise.reject(new TypeError("throw is not a method"));
      }
      return call(m, args, true);
    },
  };
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
$dump.p1 = $dump.pending(1, "later", ["s2"]);
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
$dump.n2 = ["s1", $dump.p1];
$dump.i3 = $dump.advance(Array.prototype.values.call($dump.n2), 2);
$dump.p4 = Promise.prototype.then.call($dump.p1, (v) => ({ value: v, done: false }), $dump.closeThrow($dump.i3));
$dump.i5 = $dump.fromSync($dump.i3, $dump.builtin("%ArrayIteratorPrototype%.next"));
$dump.a6 = $dump.resume(main, { at: "for-await.next#1", awaiting: $dump.p4, bindings: { out: ["s1"], x: "s1" }, loops: [{ kind: "for-await", iterator: $dump.i5, next: $dump.i5.next }] });
