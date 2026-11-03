// call.args: a call with a spread, the argument array so far.
const o = { f(...a) { return a.join(); } };
async function main() { return o.f(...[1, 2], await later(3), 4); }
main().then(print);
