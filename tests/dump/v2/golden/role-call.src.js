// call.callee, call.this, call.arg[0] below the await.
const o = { k: 1, f(a, b, c) { return [this.k, a, b, c].join(); } };
async function main() { return o.f(1, await later(2), 3); }
main().then(print);
