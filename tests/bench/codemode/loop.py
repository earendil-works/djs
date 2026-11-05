import re


async def search(query, objective):
    try:
        r = await web_search(query=query, objective=objective, max_results=5)
        results = r.get("structuredContent", {}).get("results") or []
        return f"## {query}\n" + "\n".join(f"- {x['title']} ({x.get('publishDate') or 'n.d.'}) {x['url']}\n  {re.sub(r'\s+', ' ', x['text'])[:300]}" for x in results)
    except Exception as e:
        return f"## {query}\nERR {e}"


queries = [
    ["queue retry backoff strategies", "how other systems bound retries"],
    ["session token rotation interval", "common token lifetimes"],
    ["cache invalidation by event", "event-driven invalidation patterns"],
    ["batch job scheduling fairness", "fair scheduling of batch jobs"],
    ["report rendering pipeline", "how report renderers stream output"],
    ["schema migration rollback", "safe rollback of schema changes"],
    ["upload chunk size tradeoffs", "chunk sizes for resumable uploads"],
    ["metric cardinality limits", "how metric systems cap label cardinality"],
]
parts = []
for q, o in queries:
    parts.append(await search(q, o))
text("\n\n".join(parts))
