"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(1, "later", [0]);
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
$dump.n2 = /[a-z](\d+)/g;
$dump.i3 = $dump.regexpIterator($dump.n2, "a1 b22 c333", 1);
$dump.n4 = [];
$dump.n5 = ["a1", "1"];
$dump.a6 = $dump.resume(main, { at: "await#1", awaiting: $dump.p1, slots: { matches: $dump.i3, found: $dump.n4, m: $dump.n5, it: undefined, evens: undefined, got: undefined, flat: undefined, seen: undefined, wrapped: undefined, w: undefined, cat: undefined, c: undefined, "for-of.iterator@103": $dump.i3, "for-of.next@103": $dump.builtin("%RegExpStringIteratorPrototype%.next"), "call.callee@128": Array.prototype.push, "call.this@128": $dump.n4, "binary.left@139": "1@" } });
$dump.n2.lastIndex = 2;
$dump.n5.index = 0;
$dump.n5.input = "a1 b22 c333";
$dump.n5.groups = undefined;
