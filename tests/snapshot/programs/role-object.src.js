// object.literal, object.key: a computed key's value, then a plain value.
async function main() {
  const k = "z";
  const a = { a: 1, [k]: await later(2) };
  const b = { a: 1, c: await later(3) };
  return JSON.stringify([a, b]);
}
main().then(print);
