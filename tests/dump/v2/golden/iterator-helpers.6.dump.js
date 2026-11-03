"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(9, "later", [40]);
var numbers = function* numbers() {
  let i = 0;
  while (true) yield i++;
};
async function main() {
  const matches = "a1 b22 c333".matchAll(/[a-z](\d+)/g);
  const found = [];
  for (const m of matches) found.push(m[1] + "@" + (await later(m.index)));
  print("matchAll", found.join(" "));
  const it = "x9y8".matchAll(/\d/g);
  print("first", it.next().value[0]);
  await later("between");
  print("rest", JSON.stringify([...it].map((m) => m[0])));
  const evens = numbers()
    .filter((n) => n % 2 === 0)
    .map((n) => n * 10)
    .drop(1)
    .take(3);
  const got = [];
  for (const v of evens) got.push(await later(v));
  print("helpers", got.join(" "));
  const flat = [[1, 2], [3]].values().flatMap((a) => a);
  const seen = [flat.next().value];
  await later("flat");
  seen.push(flat.next().value, flat.next().value, flat.next().done);
  print("flatMap", seen.join(" "));
  const wrapped = Iterator.from({ i: 0, next() { return { value: this.i++, done: this.i > 3 }; } });
  const w = [wrapped.next().value];
  await later("wrapped");
  w.push(wrapped.next().value);
  print("from", w.join(" "));
  const cat = Iterator.concat(["p", "q"], new Set(["r"]));
  const c = [cat.next().value];
  await later("concat");
  c.push(...cat);
  print("concat", c.join(" "));
}
$dump.i2 = $dump.regexpIterator(/[a-z](\d+)/g, "a1 b22 c333", 5);
$dump.i3 = $dump.regexpIterator(/\d/g, "x9y8", 5);
$dump.a4 = $dump.resume(numbers, { at: "yield#1", bindings: { i: 5 } });
$dump.i5 = $dump.iteratorHelper([2, 1, $dump.a4, $dump.builtin("%GeneratorPrototype%.next"), (n) => n % 2 === 0, 5, undefined, undefined]);
$dump.i6 = $dump.iteratorHelper([1, 1, $dump.i5, $dump.builtin("%IteratorHelperPrototype%.next"), (n) => n * 10, 3, undefined, undefined]);
$dump.i7 = $dump.iteratorHelper([4, 1, $dump.i6, $dump.builtin("%IteratorHelperPrototype%.next"), 0, 0, undefined, undefined]);
$dump.i8 = $dump.iteratorHelper([3, 1, $dump.i7, $dump.builtin("%IteratorHelperPrototype%.next"), 1, 0, undefined, undefined]);
$dump.n9 = [20];
$dump.a10 = $dump.resume(main, { at: "await#3", awaiting: $dump.p1, bindings: { matches: $dump.i2, found: ["1@0", "22@3", "333@7"], it: $dump.i3, evens: $dump.i8, got: $dump.n9, v: 40, flat: undefined, seen: undefined, wrapped: undefined, w: undefined, cat: undefined, c: undefined }, operands: [["call.callee", Array.prototype.push], ["call.this", $dump.n9]], loops: [{ kind: "for-of", iterator: $dump.i8, next: $dump.builtin("%IteratorHelperPrototype%.next") }] });
