// binary.left: the left operand of `*` and of `<`.
async function main() {
  const a = 5;
  return [a * (await later(6)), a < (await later(9))].join();
}
main().then(print);
