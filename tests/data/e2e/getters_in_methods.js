const o = { n: 1 };
console.log([3, 4].map(function (x) { return this.n + x; }, o));
