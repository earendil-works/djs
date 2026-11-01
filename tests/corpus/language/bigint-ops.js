// BigInt operators on pseudo-random operands (0046), checked against qjs. Comparisons of
// a negative BigInt with a negative Number are left out: qjs gets them wrong.
let seed = 0x9e3779b9;
function rand32() {
  seed = (seed + 0x6d2b79f5) | 0;
  let t = seed;
  t = Math.imul(t ^ (t >>> 15), t | 1);
  t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
  return (t ^ (t >>> 14)) >>> 0;
}
function big(limbs) {
  let v = 0n;
  const n = rand32() % (limbs + 1);
  for (let i = 0; i < n; i++) v = (v << 32n) | BigInt([0, 0xffffffff, rand32(), rand32()][rand32() % 4]);
  return rand32() & 1 ? -v : v;
}
function attempt(f) {
  try {
    return String(f());
  } catch (e) {
    return e.name + ": " + e.message;
  }
}
for (let i = 0; i < 60; i++) {
  const a = big(4);
  const b = i % 3 === 0 ? big(1) : big(3);
  const s = BigInt(rand32() % 100) - 20n;
  console.log(a + b, a - b, a * b, attempt(() => a / b), attempt(() => a % b));
  console.log(a & b, a | b, a ^ b, ~a, -a, a << s, a >> s, a < b, a <= b, a == b, a === a);
  console.log(BigInt.asIntN(64, a), BigInt.asUintN(33, a), a.toString(36), Number(a), a > 1.5, a == Number(a));
}
for (let i = 0; i < 20; i++) {
  const a = big(2);
  const e = BigInt(rand32() % 30);
  console.log(a ** e, attempt(() => a ** -e), attempt(() => (a * 2n + 1n) ** 3n / (a - 1n)));
}
let acc = 1n;
for (let i = 1n; i <= 120n; i++) acc = acc * i + (acc % 7n);
console.log(acc, acc.toString(2).length, acc % 1000000007n);
