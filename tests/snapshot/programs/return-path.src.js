// A generator's return(v) leaving a for await from its head's target: the loop's
// iterator is closed (its return() awaits) and then a finally block runs, both with the
// return completion pending (D2733).
async function* source(tag) {
  try {
    yield tag + "1";
    yield tag + "2";
  } finally {
    print(tag, "closing", await later(tag + " cleanup"));
  }
}
async function* g() {
  const o = {};
  try {
    for await (o[yield "key?"] of source("a")) print("never");
  } finally {
    print("g finally", await later("g cleanup"));
  }
}
async function main() {
  const it = g();
  print(JSON.stringify(await it.next()));
  print(JSON.stringify(await it.return("done")));
  print("end");
}
main();
