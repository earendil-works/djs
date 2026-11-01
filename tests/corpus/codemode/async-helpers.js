// Codemode scripts with their own async helpers, retries, generators and for await
// over tool results (0041).
async function fetchAll(paths) {
  const out = [];
  for (const path of paths) {
    const r = await tools.echo({ path, size: path.length });
    out.push(r);
  }
  return out;
}
const withRetry = async (fn, tries = 3) => {
  let last;
  for (let i = 0; i < tries; i++) {
    try {
      return await fn(i);
    } catch (e) {
      last = e;
      console.log("attempt", i, "failed:", e.message);
    }
  }
  throw last;
};
const files = await fetchAll(["a.ts", "bb.ts"]);
console.log(files.map((f) => f.path + ":" + f.size).join(", "));
const ok = await withRetry((i) => (i < 2 ? tools.fail({ message: "flaky " + i }) : tools.echo("third time")));
console.log(ok);
try {
  await withRetry(() => tools.fail({ message: "always" }), 2);
} catch (e) {
  console.log("gave up:", e.message);
}
function* chunks(list, n) {
  for (let i = 0; i < list.length; i += n) yield list.slice(i, i + n);
}
async function* results(list) {
  for (const chunk of chunks(list, 2)) {
    yield await Promise.all(chunk.map((x) => tools.echo(x * 10)));
  }
}
const seen = [];
for await (const batch of results([1, 2, 3, 4, 5])) seen.push(batch.join("+"));
console.log(seen);
const settled = await Promise.allSettled([tools.echo("x"), tools.fail({ message: "no" })]);
return settled.map((s) => s.status);
