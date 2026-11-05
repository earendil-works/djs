// The tail: about a hundred calls in one body (a scan over many files with per-file
// regex work, then a summary near the output limit), as in the largest 1% of bodies.
const list = (await tools.bash({ command: "ls -R src lib | head -200", timeout: 30 })).output;
const dirs = ["src/server", "src/client", "src/core", "src/util", "src/api", "lib"];
const paths = [];
for (let i = 0; i < 96; i++) paths.push(`${dirs[i % dirs.length]}/module-${i}.ts`);
const stats = [];
for (let i = 0; i < paths.length; i += 8) {
  const batch = paths.slice(i, i + 8);
  const texts = await Promise.all(batch.map((path) => tools.read({ path })));
  batch.forEach((path, j) => {
    const t = texts[j];
    const lines = t.split("\n");
    const fns = (t.match(/function \w+/g) || []).length;
    const todos = lines.filter((l) => l.includes("TODO") || l.includes("FIXME"));
    const longest = lines.reduce((m, l) => Math.max(m, l.length), 0);
    stats.push({ path, lines: lines.length, fns, todos: todos.length, longest, first: todos[0]?.trim().slice(0, 80) ?? "" });
  });
}
stats.sort((a, b) => b.todos - a.todos || b.lines - a.lines);
const total = stats.reduce((s, x) => s + x.lines, 0);
let out = `${list.split("\n").length} entries listed, ${stats.length} files, ${total} lines\n`;
for (const s of stats) out += `${s.path.padEnd(28)} ${String(s.lines).padStart(5)} lines ${String(s.fns).padStart(3)} fns ${String(s.todos).padStart(2)} todo  ${s.first}\n`;
return out.slice(0, 40000);
