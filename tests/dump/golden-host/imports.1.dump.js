"djs-dump";
$djs.k(async function (linear, tracker, held) {
const hits = await $djs.resume(1, "@mcp/github#search_issues", [{ q: "imports" }]);
text(`${hits.length} hits`);
const [made, all] = await Promise.all([held({ title: hits[0].title }), linear.list({ path: "open" })]);
text(made);
return [all, typeof tracker, held === linear.create_issue, Object.keys(linear)];
}, $djs.source("imports.js", "673d314f", "search_issues,search,linear,tracker"))($djs.host("@mcp/linear"), $djs.host("tools.list"), $djs.host("@mcp/linear#create_issue"));
