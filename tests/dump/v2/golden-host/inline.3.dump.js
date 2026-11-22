"djs-dump";
globalThis.$dump = {};
$dump.n2 = (name, args) => tools[name](args);
$dump.a3 = $djs.k(async function (search_issues, results) {
const issue = await $djs.resume(5, "@mcp/github#search_issues", [{ q: "<read.md>,[list.md]" }]);
return [results, issue, typeof search_issues];
}, (async function (search_issues) {
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

}), 2)($djs.host("@mcp/github#search_issues"), ["<read.md>", "[list.md]"]);
Object.defineProperty($dump.n2, "name", { value: "call" });
$dump.a3;
