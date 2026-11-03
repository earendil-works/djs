// array.element[i] and, after a hole, array.literal.
async function main() {
  const a = [1, 2, await later(3)];
  const b = [1, , await later(3)];
  return a.join() + " " + b.length;
}
main().then(print);
