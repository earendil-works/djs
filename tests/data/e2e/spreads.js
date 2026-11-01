function sum(a, b, c, d) { return a + b + c + (d === undefined ? 0 : d); }
const xs = [1, 2];
console.log(sum(...xs, 3), sum(0, ...xs, ...[10]), [...xs, 3, ...[]], [..."a😀b"].length);
function P(a, b) { this.v = a * b; }
console.log(new P(...[6, 7]).v, [...[1, , 3]]);
const src = { a: 1, b: 2 };
const merged = { ...src, b: 3, ...{ c: 4 } };
console.log(merged);
try { [...1]; } catch (e) { console.log(e.name, e.message); }
