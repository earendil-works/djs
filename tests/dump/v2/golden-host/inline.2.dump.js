"djs-dump";
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf([][Symbol.iterator]())["next"];
$dump.n3 = (name, args) => tools[name](args);
$dump.a5 = $djs.k(async function (search_issues, results, call) {
const $djs_L = $djs.L;
{ let $djs_f1 = true; for (const $djs_v1 of $djs_L($djs.advance(Array.prototype.values.call($djs.host("ALL_TOOLS")), 3), $dump.b0)) { if ($djs_f1) { $djs_f1 = false; {
const t = $djs.host("ALL_TOOLS")["2"];
results.push(await $djs.resume(3, "tools.list", [{ path: "list.md" }]));
text(`${t.name} done`);
} } else { const t = $djs_v1; {
  if (t.name === "ask") continue;
  results.push(await call(t.name, { path: `${t.name}.md` }));
  text(`${t.name} done`);
} } } }
const issue = await search_issues({ q: results.join(",") });
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

}), 1, 50)($djs.host("@mcp/github#search_issues"), ["<read.md>"], $dump.n3);
Object.defineProperty($dump.n3, "name", { value: "call" });
$dump.a5;
