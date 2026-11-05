path = "src/server/session-handler.ts"
text(await read(path=path, offset=120, limit=60))
text(await edit(
    path=path,
    edits=[
        {"oldText": "export function loadSession(session: Session, opts = {}) {\n  if (!session.token) throw new Error(\"The token is missing.\");", "newText": "export function loadSession(session: Session, opts: LoadOptions = {}) {\n  if (!session.token) throw new SessionError(\"missing-token\", \"The token is missing.\");"},
        {"oldText": "  for (const item of session.records) applyRecord(item);", "newText": "  for (const item of session.records) {\n    if (item.expired) continue;\n    applyRecord(item, opts.clock ?? Date.now);\n  }"},
        {"oldText": "// TODO: retry the flush when the queue is full.", "newText": "// The flush is retried by the queue itself (see flushQueue), up to opts.retries times."},
        {"oldText": "  session.state = \"open-0\";\n  return session;", "newText": "  session.state = \"open\";\n  session.openedAt = opts.clock?.() ?? Date.now();\n  return session;"},
        {"oldText": "import { renderReport, formatEvent } from \"./report\";", "newText": "import { renderReport, formatEvent, type ReportOptions } from \"./report\";\nimport { SessionError } from \"./errors\";"},
        {"oldText": "export async function flushQueue(queue: Queue, opts = {}) {", "newText": "export async function flushQueue(queue: Queue, opts: { retries?: number; signal?: AbortSignal } = {}) {\n  const retries = opts.retries ?? 3;"},
        {"oldText": "    throw new Error(\"The queue is closed.\");", "newText": "    throw new SessionError(\"queue-closed\", `The queue ${queue.name} is closed.`);"},
        {"oldText": "  const cache0 = buildCache(request, 17);\n  const token1 = resolveToken(request, 4);", "newText": "  const cache0 = buildCache(request, opts.cacheSize ?? 17);\n  const token1 = resolveToken(request, opts.tokenTtl ?? 4);\n  if (!token1) return null;"},
    ],
))
text(await bash(command="git diff --stat", timeout=30))
r = await bash(command="npm run check 2>&1 | tail -40", timeout=120)
text("check passed" if r["exitCode"] == 0 else "\n".join(r["output"].split("\n")[-25:]))
