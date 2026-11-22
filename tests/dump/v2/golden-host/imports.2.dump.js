"djs-dump";
$djs.k(async function (linear, tracker, held) {
const [made, all] = await Promise.all([$djs.resume(3, "@mcp/linear#create_issue", [{ title: "issue about imports" }]), $djs.resume(4, "@mcp/linear#list", [{ path: "open" }])]);
text(made);
return [all, typeof tracker, held === linear.create_issue, Object.keys(linear)];
}, $djs.source("imports.js", "673d314f", "search_issues,search,linear,tracker"), 2)($djs.host("@mcp/linear"), $djs.host("tools.list"), $djs.host("@mcp/linear#create_issue"));
