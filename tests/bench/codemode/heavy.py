import asyncio
import re

lst = (await bash(command="ls -R src lib | head -200", timeout=30))["output"]
dirs = ["src/server", "src/client", "src/core", "src/util", "src/api", "lib"]
paths = [f"{dirs[i % len(dirs)]}/module-{i}.ts" for i in range(96)]
stats = []
for i in range(0, len(paths), 8):
    batch = paths[i:i + 8]
    texts = await asyncio.gather(*[read(path=p) for p in batch])
    for path, t in zip(batch, texts):
        lines = t.split("\n")
        fns = len(re.findall(r"function \w+", t))
        todos = [l for l in lines if "TODO" in l or "FIXME" in l]
        longest = max((len(l) for l in lines), default=0)
        stats.append({"path": path, "lines": len(lines), "fns": fns, "todos": len(todos), "longest": longest, "first": todos[0].strip()[:80] if todos else ""})
stats.sort(key=lambda x: (-x["todos"], -x["lines"]))
total = sum(x["lines"] for x in stats)
out = f"{len(lst.split(chr(10)))} entries listed, {len(stats)} files, {total} lines\n"
for s in stats:
    out += f"{s['path'].ljust(28)} {str(s['lines']).rjust(5)} lines {str(s['fns']).rjust(3)} fns {str(s['todos']).rjust(2)} todo  {s['first']}\n"
__result = out[:40000]
