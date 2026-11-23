"djs-dump";
$djs.k(async function (search_issues, read) {
const [a, b] = await Promise.all([$djs.resume(2, "tools.read", [{ path: "a.txt" }]), $djs.resume(3, "tools.read", [{ path: "b.txt", at: "1970-01-01T00:00:00.000Z" }])]);
text(a + b);
store("seen", { files: new Set(["a.txt"]), n: 2, skip: undefined });
const hits = await search_issues({ q: "dumps" });
let answer;
try {
  answer = await tools.ask({ question: `open ${hits[0].title}?` });
} catch (e) {
  text(`refused: ${e.message} ${JSON.stringify(e.structuredOutput)}`);
  answer = await read({ path: "fallback.txt" });
}
text(String(read === tools.read && ALL_TOOLS[0].name === "read"));
return { answer, hits: hits.length, load: load("seen") };
}, $djs.source("tools.js", "e367ea6a", "search_issues"))($djs.host("@mcp/github#search_issues"), $djs.host("tools.read"));
