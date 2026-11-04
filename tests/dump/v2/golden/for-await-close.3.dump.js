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
$dump.a3 = $dump.resume(source, { at: "await#1", awaiting: $dump.p1, args: ["c"], slots: { tag: "c", "completion.kind@32": 2, "completion.value@32": undefined, "call.callee@98": print, "call.this@98": undefined, "call.arg@104": "c", "call.arg@109": "closing" }, queue: [{ type: "return", value: undefined, promise: $dump.p2 }] });
$dump.a4 = $dump.resume(firstOf, { at: "for-await.close#2", awaiting: $dump.p2, args: ["c"], slots: { tag: "c", v: "c1", "completion.value@32": "first c1" } });
$dump.a5 = $dump.resume(main, { at: "await#1", awaiting: $dump.a4, slots: { "call.callee@277": print, "call.this@277": undefined } });
