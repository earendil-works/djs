// Math.random's state goes with the dump (`$dump.random`): the sequence continues.
async function main() {
  const a = Math.random();
  await later(0);
  const b = Math.random();
  await later(1);
  print(a, b, Math.random());
}
main();
