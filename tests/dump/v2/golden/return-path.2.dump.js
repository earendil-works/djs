"djs-dump 2";
$dump.random(2135587861, -1640531527);
$dump.p1 = $dump.pending(4, "later", ["g cleanup"]);
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
$dump.a3 = $dump.resume(g, { at: "await#1", awaiting: $dump.p1, slots: { o: {}, "completion.kind@40": 2, "completion.value@40": "done", "call.callee@127": print, "call.this@127": undefined, "call.arg@133": "g finally" }, queue: [{ type: "return", value: "done", promise: $dump.p2 }] });
$dump.a4 = $dump.resume(main, { at: "await#2", awaiting: $dump.p2, slots: { it: $dump.a3, "call.callee@86": print, "call.this@86": undefined, "call.callee@92": JSON.stringify, "call.this@92": JSON } });
