// A sequence of roles: a call's arguments, an array, an object and a template, nested.
const o = { f(a, b) { return a + "|" + JSON.stringify(b); } };
async function main() {
  const p = "x";
  return o.f(p + "!", [1, { k: `v${2}w${await later(3)}` }]);
}
main().then(print);
