"djs-dump";
$djs.k(async function (read, hits, answer) {
try {
(answer = await $djs.resume(7, "tools.ask", [{ question: "open issue about dumps?" }]));
} catch (e) {
  text(`refused: ${e.message} ${JSON.stringify(e.structuredOutput)}`);
  answer = await read({ path: "fallback.txt" });
}
text(String(read === tools.read && ALL_TOOLS[0].name === "read"));
return { answer, hits: hits.length, load: load("seen") };
}, $djs.source("tools.js", "e367ea6a", "search_issues"), 3)($djs.host("tools.read"), [{ title: "issue about dumps" }], undefined);
