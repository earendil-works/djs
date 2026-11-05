// Promise.all over a list of reads and commands, the results joined under headers and cut
// to an output budget (about 22% of the bodies).
const files = [
  "src/server/session.ts", "src/server/queue.ts", "src/server/handler-route.ts", "src/core/cache.ts",
  "src/core/schema.ts", "src/core/index-token.ts", "src/api/client.ts", "src/api/report.ts",
  "src/util/format.ts", "src/util/encode.ts", "lib/worker.js", "docs/config.md",
];
const bodies = await Promise.all(files.map((path) => tools.read({ path, offset: null, limit: null })));
const checks = await Promise.allSettled([
  tools.bash({ command: "npm run lint -- src/server", timeout: 60 }),
  tools.bash({ command: "npm run test -- --run src/core", timeout: 120 }),
  tools.read({ path: "/missing/old-config.json" }),
]);
let out = files.map((f, i) => `=== ${f} (${bodies[i].split("\n").length} lines)\n${bodies[i]}`).join("\n");
for (const c of checks) out += "\n--- " + (c.status === "fulfilled" ? (typeof c.value === "string" ? c.value : c.value.output).slice(-2000) : "failed: " + c.reason.message);
return out.length > 40000 ? out.slice(0, 40000) + `\n[${out.length - 40000} more characters]` : out;
