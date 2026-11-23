"djs-dump";
$djs.k(async function (read) {
const hits = await $djs.resume(6, "@mcp/github#search_issues", [{ q: "dumps" }]);
let answer;
try {
  answer = await tools.ask({ question: `open ${hits[0].title}?` });
} catch (e) {
  text(`refused: ${e.message} ${JSON.stringify(e.structuredOutput)}`);
  answer = await read({ path: "fallback.txt" });
}
text(String(read === tools.read && ALL_TOOLS[0].name === "read"));
return { answer, hits: hits.length, load: load("seen") };
}, $djs.source("tools.js", "e367ea6a", "search_issues"), 2)($djs.host("tools.read"));
