"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(2, "later", ["a cleanup"]);
var source = async function* source(tag) {
  try {
    yield tag + "1";
    yield tag + "2";
  } finally {
    print(tag, "closing", await later(tag + " cleanup"));
  }
};
var g = async function* g() {
  const o = {};
  try {
    for await (o[yield "key?"] of source("a")) print("never");
  } finally {
    print("g finally", await later("g cleanup"));
  }
};
async function main() {
  const it = g();
  print(JSON.stringify(await it.next()));
  print(JSON.stringify(await it.return("done")));
  print("end");
}
$dump.p2 = new Promise(() => {});
$dump.a3 = $dump.resume(source, { at: "await#1", awaiting: $dump.p1, args: ["a"], slots: { tag: "a", "completion.kind@32": 2, "completion.value@32": undefined, "call.callee@98": print, "call.this@98": undefined, "call.arg@104": "a", "call.arg@109": "closing" }, queue: [{ type: "return", value: undefined, promise: $dump.p2 }] });
$dump.p4 = new Promise(() => {});
$dump.a5 = $dump.resume(g, { at: "for-await.close#2", awaiting: $dump.p2, slots: { o: {}, "completion.value@50": "done" }, queue: [{ type: "return", value: "done", promise: $dump.p4 }] });
$dump.a6 = $dump.resume(main, { at: "await#2", awaiting: $dump.p4, slots: { it: $dump.a5, "call.callee@86": print, "call.this@86": undefined, "call.callee@92": JSON.stringify, "call.this@92": JSON } });
