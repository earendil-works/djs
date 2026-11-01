// Typical codemode: fan out tool calls and aggregate.
const files = ["a.ts", "b.ts", "c.ts"];
const results = await Promise.all(files.map((path, i) => tools.echo({ path, lines: (i + 1) * 10 })));
const total = results.reduce((sum, r) => sum + r.lines, 0);
for (const r of results) console.log(`${r.path}: ${r.lines} lines`);
const settled = await Promise.allSettled([tools.echo(1), tools.fail({ message: "bad" })]);
console.log(settled.map((s) => s.status + ":" + (s.status === "fulfilled" ? s.value : s.reason.message)));
const first = await Promise.race([tools.echo("fast"), new Promise(() => {})]);
console.log(first);
return { total, count: results.length };
