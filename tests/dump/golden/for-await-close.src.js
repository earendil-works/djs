// Closing an async iterator while its return() waits: a for await left by break, by an
// exception and by a return (through the loop's close subroutine), each over an async
// generator whose finally awaits.
async function* source(tag) {
  try {
    yield tag + "1";
    yield tag + "2";
  } finally {
    print(tag, "closing", await later(tag + " cleanup"));
  }
}
async function firstOf(tag) {
  for await (const v of source(tag)) {
    return "first " + v;
  }
}
async function main() {
  for await (const v of source("a")) {
    print("got", v);
    break;
  }
  try {
    for await (const v of source("b")) {
      print("got", v);
      throw new Error("body failed on " + v);
    }
  } catch (e) {
    print("caught", e.message);
  }
  print(await firstOf("c"));
  print("end");
}
main();
