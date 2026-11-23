// new.callee, new.arg[0]; and new.args with a spread.
class Thing { constructor(...a) { this.a = a; } }
async function main() {
  const x = new Thing(1, await later(2)).a.join();
  const y = new Thing(...[1], await later(2), 3).a.join();
  return x + " " + y;
}
main().then(print);
