"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(5, "later", ["b cleanup"]);
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
$dump.a3 = $dump.resume(source, { at: "await#1", awaiting: $dump.p1, args: ["b"], slots: { tag: "b", "completion.kind@32": 2, "completion.value@32": undefined, "call.callee@98": print, "call.this@98": undefined, "call.arg@104": "b", "call.arg@109": "closing" }, queue: [{ type: "return", value: undefined, promise: $dump.p2 }] });
$dump.p4 = Promise.prototype.then.call($dump.p2, () => (undefined), () => (undefined));
$dump.n5 = new Error("body failed on b1");
$dump.a6 = $dump.resume(main, { at: "for-await.close#3", awaiting: $dump.p4, slots: { v: "b1", "for-await.exception@111": $dump.n5 } });
Object.defineProperty($dump.n5, "stack", { value: "    at main (src.js:25:22)\n", writable: true, configurable: true });
