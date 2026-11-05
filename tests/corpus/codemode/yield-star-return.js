// An async generator's return() through `yield*` while the inner return() is awaited:
// a done result returns through the outer `finally`, one that is not yields and the
// delegation goes on (plain dumps, 0203).
async function* inner(tag) {
  try { yield tag + 1; yield tag + 2; } finally { console.log("inner finally", tag, await tools.echo(tag)); }
}
async function* outer(src) {
  try {
    const r = yield* src;
    console.log("delegated", r);
  } finally {
    console.log("outer finally", await tools.echo("f"));
  }
  return "end";
}
function custom() {
  let n = 0;
  return {
    [Symbol.asyncIterator]() { return this; },
    next() { return tools.echo({ value: "c" + n++, done: false }); },
    return(v) {
      console.log("custom return", v);
      return tools.echo(n < 3 ? { value: "kept" + n++, done: false } : { value: v + "!", done: true });
    },
  };
}
const g = outer(inner("a"));
console.log(JSON.stringify(await g.next()));
console.log(JSON.stringify(await g.return("stop")));
console.log(JSON.stringify(await g.next()));
const h = outer(custom());
console.log(JSON.stringify(await h.next()));
for (const v of ["x", "y", "z"]) console.log(JSON.stringify(await h.return(v)));
console.log(JSON.stringify(await h.next()));
