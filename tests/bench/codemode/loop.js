// A for-of loop with one awaited call per item, each result formatted into Markdown
// (about 8% of the bodies).
async function search(query, objective) {
  try {
    const r = await tools.web_search({ query, objective, max_results: 5 });
    return `## ${query}\n` + (r.structuredContent?.results || []).map((x) => `- ${x.title} (${x.publishDate || "n.d."}) ${x.url}\n  ${x.text.replace(/\s+/g, " ").slice(0, 300)}`).join("\n");
  } catch (e) {
    return `## ${query}\nERR ${e}`;
  }
}
const queries = [
  ["queue retry backoff strategies", "how other systems bound retries"],
  ["session token rotation interval", "common token lifetimes"],
  ["cache invalidation by event", "event-driven invalidation patterns"],
  ["batch job scheduling fairness", "fair scheduling of batch jobs"],
  ["report rendering pipeline", "how report renderers stream output"],
  ["schema migration rollback", "safe rollback of schema changes"],
  ["upload chunk size tradeoffs", "chunk sizes for resumable uploads"],
  ["metric cardinality limits", "how metric systems cap label cardinality"],
];
const parts = [];
for (const [q, o] of queries) parts.push(await search(q, o));
text(parts.join("\n\n"));
