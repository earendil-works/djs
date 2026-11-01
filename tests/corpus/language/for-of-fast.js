// for-of and array destructuring over Array / String / TypedArray iterators: the
// interpreter skips the result objects while `next` is the original (0080, D1800).
// Everything here must behave exactly as the full protocol does.
const out = [];
const log = (...a) => out.push(a.join(" "));

// Holes read through the prototype chain; getters on elements run.
{
  const a = [1, , 3];
  Object.prototype[1] = "proto";
  const r = [];
  for (const x of a) r.push(x);
  delete Object.prototype[1];
  for (const x of a) r.push(x);
  const b = [0, 1];
  Object.defineProperty(b, 1, { get() { r.push("get"); return 7; } });
  for (const x of b) r.push(x);
  log("holes", r.join());
}

// Mutation during iteration: growth, shrinking, length reset.
{
  const a = [1, 2, 3];
  const r = [];
  let once = true;
  for (const x of a) {
    r.push(x);
    if (x === 1) a.push(4);
    if (x === 2) a.length = 3;
    if (x === 3 && once) { once = false; a.unshift(0); }
  }
  log("mutate", r.join(), a.join());
  const s = [1, 2, 3, 4];
  const r2 = [];
  for (const x of s) { r2.push(x); s.pop(); }
  log("shrink", r2.join());
}

// keys / entries / strings / typed arrays.
{
  const a = ["a", "b"];
  const r = [];
  for (const k of a.keys()) r.push(k);
  for (const [i, v] of a.entries()) r.push(i + v);
  for (const c of "x\u{1F600}y\uD800") r.push(c.length);
  for (const x of new Int16Array([-1, 2])) r.push(x);
  for (const x of new BigInt64Array([5n])) r.push(typeof x);
  log("kinds", r.join());
}

// Replacing next after the loop started has no effect; before, it does.
{
  const r = [];
  const a = [1, 2, 3];
  const proto = Object.getPrototypeOf([][Symbol.iterator]());
  const next = proto.next;
  for (const x of a) {
    r.push(x);
    proto.next = function () { return { done: true }; };
  }
  for (const x of a) r.push("never" + x);
  proto.next = function () { r.push("mine"); return next.call(this); };
  for (const x of [8, 9]) r.push(x);
  const [p, q] = [5, 6];
  r.push(p, q);
  proto.next = next;
  log("next", r.join());
}

// A custom @@iterator on the array or on Array.prototype.
{
  const r = [];
  const a = [1, 2];
  a[Symbol.iterator] = function* () { yield "own"; };
  for (const x of a) r.push(x);
  const values = Array.prototype[Symbol.iterator];
  Array.prototype[Symbol.iterator] = function* () { yield "proto"; };
  for (const x of [1]) r.push(x);
  const [d] = [2];
  r.push(d);
  Array.prototype[Symbol.iterator] = values;
  const sv = String.prototype[Symbol.iterator];
  String.prototype[Symbol.iterator] = function* () { yield "str"; };
  for (const x of "ab") r.push(x);
  String.prototype[Symbol.iterator] = sv;
  log("iterator", r.join());
}

// Destructuring closes the iterator: a `return` anywhere on the chain is called.
{
  const r = [];
  const ip = Object.getPrototypeOf(Object.getPrototypeOf([][Symbol.iterator]()));
  ip.return = function () { r.push("return"); return {}; };
  const [a] = [1, 2];
  for (const x of [1, 2]) { r.push(x); break; }
  for (const x of [3]) r.push(x);
  delete ip.return;
  Object.prototype.return = function () { r.push("oproto"); return {}; };
  const [b] = [1, 2];
  delete Object.prototype.return;
  Object.defineProperty(ip, "return", { get() { r.push("getter"); return undefined; }, configurable: true });
  const [c] = [1, 2];
  delete ip.return;
  const [e] = [1, 2];
  r.push(a, b, c, e);
  log("close", r.join());
}

// Detached typed arrays throw from next; a proxy target goes through its traps.
{
  const ta = new Uint8Array(new ArrayBuffer(4, { maxByteLength: 8 }));
  const r = [];
  try {
    for (const x of ta) { r.push(x); ta.buffer.resize(0); }
  } catch (e) { r.push(e.constructor.name); }
  const p = new Proxy([1, 2], { get(t, k, rcv) { if (typeof k === "string") r.push("get " + k); return Reflect.get(t, k, rcv); } });
  for (const x of Array.prototype.values.call(p)) r.push(x);
  for (const x of p) r.push(x);
  log("exotic", r.join());
}

// The iterator object is the usual one, and a finished one stays done.
{
  const it = [1, 2][Symbol.iterator]();
  const r = [];
  for (const x of it) { r.push(x); break; }
  for (const x of it) r.push(x);
  for (const x of it) r.push("again");
  r.push(JSON.stringify(it.next()), Object.prototype.toString.call(it));
  const sit = "ab"[Symbol.iterator]();
  for (const c of sit) r.push(c);
  r.push(JSON.stringify(sit.next()));
  log("iterator-object", r.join());
}

// Errors in the body close the iterator and keep their identity.
{
  const r = [];
  try {
    for (const x of [1, 2]) throw new Error("body " + x);
  } catch (e) { r.push(e.message); }
  let n = 0;
  for (const [i, [a, b]] of [[1, 2], [3, 4]].entries()) n += i + a + b;
  r.push(n);
  log("misc", r.join());
}
console.log(out.join("\n"));
