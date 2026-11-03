"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(8, "later", ["c cleanup"]);
var source = async function* source(tag) {
  try {
    yield tag + "1";
    yield tag + "2";
  } finally {
    print(tag, "closing", await later(tag + " cleanup"));
  }
};
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
$dump.p2 = new Promise(() => {});
$dump.a3 = $dump.resume(source, { at: "await#1", awaiting: $dump.p1, args: ["c"], bindings: { tag: "c" }, operands: [["finally.value", undefined], ["finally.return", 0], ["call.callee", print], ["call.this", undefined], ["call.arg[0]", "c"], ["call.arg[1]", "closing"]], queue: [{ type: "return", value: undefined, promise: $dump.p2 }] });
$dump.a4 = $dump.resume(firstOf, { at: "for-await.close#2", awaiting: $dump.p2, args: ["c"], bindings: { tag: "c", v: "c1" }, operands: [["finally.value", "first c1"], ["finally.return", 0]] });
$dump.a5 = $dump.resume(main, { at: "await#1", awaiting: $dump.a4, operands: [["call.callee", print], ["call.this", undefined]] });
