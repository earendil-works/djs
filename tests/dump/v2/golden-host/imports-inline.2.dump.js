"djs-dump";
$djs.k(async function (linear, tracker, held) {
const [made, all] = await Promise.all([$djs.resume(3, "@mcp/linear#create_issue", [{ title: "issue about imports" }]), $djs.resume(4, "@mcp/linear#list", [{ path: "open" }])]);
text(made);
return [all, typeof tracker, held === linear.create_issue, Object.keys(linear)];
}, (async function (search_issues, search, linear, tracker) {
                                                                         
                                                      
                                      
                                  
const held = linear.create_issue;
const hits = await search({ q: "imports" });
text(`${hits.length} hits`);
const [made, all] = await Promise.all([held({ title: hits[0].title }), linear.list({ path: "open" })]);
text(made);
return [all, typeof tracker, held === linear.create_issue, Object.keys(linear)];

}), 2)($djs.host("@mcp/linear"), $djs.host("tools.list"), $djs.host("@mcp/linear#create_issue"));
