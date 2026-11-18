// Ints formatted without the float formatter, toFixed's 128-bit path and its edges, and
// charCodeAt on a rope read again after its first flattening.
const ints = [0, -0, 1, -1, 9, 10, -10, 99, 100, 2147483647, -2147483648, 2147483648, -2147483649, 4294967295, 4294967296, 1e15, -1e15, 2 ** 53, -(2 ** 53), 1e21, 123456789];
console.log(ints.map((i) => String(i)).join(" "));
console.log(ints.map((i) => "" + i + `|${i}`).join(" "));
const parts = [];
for (let i = -1005; i <= 1005; i += 67) parts.push(String(i), (i | 0).toString(), `${i * 1000003 | 0}`);
console.log(parts.join(","));
const fx = [];
for (const x of [0, -0, 0.5, 1.5, 2.5, -2.5, 1.005, 1.045, 0.125, 0.375, 10.235, 123.456, -0.0049, 0.0005, 1 / 3, 2 / 3, 1e-10, 5e-324, 9007199254740991.5, 9007199254740993, 4503599627370495.5, 1e20, 999999999999999.9, 1.7976931348623157e308])
  for (const f of [0, 1, 2, 3, 5, 10, 20, 21, 22, 23, 50, 100]) fx.push(x.toFixed(f));
console.log(fx.join(" "));
let sum = 0;
for (let i = 0; i < 2000; i++) sum += (i / 7).toFixed(i % 23).length;
console.log(sum);
const t = "function hello(world) { return world + 1; }\n".repeat(200);
let n = 0;
for (let r = 0; r < 3; r++) for (let i = 0; i < t.length; i += 7) n += t.charCodeAt(i);
console.log(n, t.slice(40, 60), (t + "!").charCodeAt(t.length));
