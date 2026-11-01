// A sequential loop with an await per iteration; keeps per-file summaries.
const hits = await tools.search({ query: "TODO", limit: 40 });
const summary = [];
let total = 0;
for (const hit of hits) {
  const file = await tools.read({ path: hit.path });
  const todos = file.text
    .split("\n")
    .map((text, i) => ({ line: i + 1, text: text.trim() }))
    .filter((l) => l.text.includes("pub fn"));
  total += todos.length;
  summary.push({ path: hit.path, count: todos.length, first: todos[0]?.text ?? null });
}
summary.sort((a, b) => b.count - a.count);
text(summary.slice(0, 5).map((s) => `${s.path}: ${s.count}`).join("\n"));
await tools.ask({ question: `Found ${total} matches. File issues?`, options: ["yes", "no"] });
return total;
