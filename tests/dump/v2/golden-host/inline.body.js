// A durable code mode body dumped with its text inlined: it loads with no sources.
const results = [];
const call = (name, args) => tools[name](args);
for (const t of ALL_TOOLS) {
  if (t.name === "ask") continue;
  results.push(await call(t.name, { path: `${t.name}.md` }));
  text(`${t.name} done`);
}
const issue = await search_issues({ q: results.join(",") });
return [results, issue, typeof search_issues];
