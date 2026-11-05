import asyncio

files = [
    "src/server/session.ts", "src/server/queue.ts", "src/server/handler-route.ts", "src/core/cache.ts",
    "src/core/schema.ts", "src/core/index-token.ts", "src/api/client.ts", "src/api/report.ts",
    "src/util/format.ts", "src/util/encode.ts", "lib/worker.js", "docs/config.md",
]
bodies = await asyncio.gather(*[read(path=p, offset=None, limit=None) for p in files])


async def settle(coro):
    try:
        return {"status": "fulfilled", "value": await coro}
    except Exception as e:
        return {"status": "rejected", "reason": str(e)}


checks = await asyncio.gather(
    settle(bash(command="npm run lint -- src/server", timeout=60)),
    settle(bash(command="npm run test -- --run src/core", timeout=120)),
    settle(read(path="/missing/old-config.json")),
)
out = "\n".join(f"=== {f} ({len(bodies[i].split(chr(10)))} lines)\n{bodies[i]}" for i, f in enumerate(files))
for c in checks:
    if c["status"] == "fulfilled":
        v = c["value"]
        out += "\n--- " + (v if isinstance(v, str) else v["output"])[-2000:]
    else:
        out += "\n--- failed: " + c["reason"]
__result = out[:40000] + f"\n[{len(out) - 40000} more characters]" if len(out) > 40000 else out
