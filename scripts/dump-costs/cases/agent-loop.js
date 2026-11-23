// A programmatic script: helpers, a retry wrapper, an async generator over pages, for await,
// try/finally, and awaits inside expressions.
const seen = new Set();
const log = [];
const sleepless = (ms) => Promise.resolve(ms);
async function retry(fn, attempts = 3) {
  let last;
  for (let i = 0; i < attempts; i++) {
    try {
      return await fn(i);
    } catch (e) {
      last = e;
      log.push(`attempt ${i} failed: ${e.message}`);
      await sleepless(10 * (i + 1));
    } finally {
      log.push(`attempt ${i} done`);
    }
  }
  throw last;
}
async function* pages(query) {
  let cursor = null;
  do {
    const page = await retry(() => tools.search_page({ query, cursor }));
    cursor = page.next;
    yield* page.items;
  } while (cursor);
}
const owners = {};
for await (const item of pages("flaky test")) {
  if (seen.has(item.id)) continue;
  seen.add(item.id);
  (owners[item.owner] ??= []).push(item.id);
  if (seen.size % 25 === 0) log.push(`processed ${seen.size}, latest ${(await tools.status({ id: item.id })).state}`);
}
const ranked = Object.entries(owners).sort((a, b) => b[1].length - a[1].length);
const summary = ranked.slice(0, 3).map(([owner, ids]) => `${owner}: ${ids.length}`);
summary.push(await tools.ask({ question: summary.join("; "), options: ["ok"] }).then((a) => a.choice));
return { summary, log: log.length };
