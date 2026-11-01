// Property reads, writes, creation and deletion on plain objects (prop_*).
// bench: expect 1000007999999
const o = { a: 1, b: 2, c: 3, d: 4 };
let sum = 0;
for (let i = 0; i < 1_000_000; i++) {
  o.a = i;
  o.b = o.a + o.c;
  sum += o.b + o.d;
  const t = { x: i, y: i + 1 };
  delete t.x;
  sum += t.y;
}
return sum + o.a;
