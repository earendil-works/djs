// A durable code mode body: tools by namespace and binding, host data, the JSON boundary.
text(`${ALL_TOOLS.length} tools`);
const read = tools.read;
const [a, b] = await Promise.all([read({ path: "a.txt" }), tools.read({ path: "b.txt", at: new Date(0) })]);
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
