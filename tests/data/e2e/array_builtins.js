const a = [1, 2, 3];
console.log(a.map(function (x) { return x * 2; }), a.indexOf(2), a.indexOf(9), a.includes(3), a.join("-"), a.join());
let sum = 0;
a.forEach(function (x, i) { sum += x * i; });
console.log(sum, [1, null, undefined, "s"].join(), [NaN].includes(NaN), [NaN].indexOf(NaN));
console.log(a.map.name, a.map.length, a.join.length, typeof a.forEach);
const sparse = [1, , 3];
let visited = "";
sparse.forEach(function (x, i) { visited += i; });
console.log(visited, sparse.map(function (x) { return x + 1; }));
const v = { valueOf: function () { return 2; } };
console.log([1, 2, 3].indexOf(2, v), [1, 2, 3].includes(1, v), [10, 20].map(function (x) { return x + v; }));
try { a.map(1); } catch (e) { console.log(e.name, e.message); }
