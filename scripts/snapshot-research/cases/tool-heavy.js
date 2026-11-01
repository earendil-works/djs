// js/bench/pi.bench.ts TOOL_HEAVY: search, read in parallel batches, filter, summarize.
const hits = await tools.search({ query: "error handling", limit: 60 });
const files = [];
for (let i = 0; i < hits.length; i += 10) {
  const batch = await Promise.all(hits.slice(i, i + 10).map((hit) => tools.read({ path: hit.path })));
  files.push(...batch);
}
const matches = files
  .map((file) => ({ path: file.path, lines: file.text.split("\n").filter((line) => line.includes("fn ")) }))
  .filter((file) => file.lines.length > 0)
  .sort((a, b) => b.lines.length - a.lines.length);
for (const file of matches.slice(0, 10)) console.log(file.path.padEnd(40), String(file.lines.length).padStart(4));
store("seen", matches.map((file) => file.path).slice(0, 20));
const review = await tools.ask({ question: "Open the top file?", options: matches.slice(0, 3).map((m) => m.path) });
return { files: files.length, matching: matches.length, top: matches[0]?.path, review: review.choice };
