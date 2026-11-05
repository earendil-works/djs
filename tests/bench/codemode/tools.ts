// The stub tools of the codemode workload set: deterministic, invented answers in the
// shapes agents' tools return (file text, command output, search results, query rows,
// images). An answer depends only on the tool's name and arguments (seeded by their hash),
// so the order of concurrent calls does not matter and every engine sees the same data.
//
// `record(body)` runs a workload body in node with these tools and returns the answers it
// asked for, keyed `name + JSON.stringify(args)` with JSON text values: the table the
// engines' stub tools read (js/bench/engines/prelude.js), the way a host bridge passes
// arguments out and results in as JSON.

export type Args = Record<string, unknown>;

/** FNV-1a over UTF-16 code units: the seed of an answer. */
export function hash(s: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 0x01000193) >>> 0;
  return h;
}

class Rng {
  private s: number;
  constructor(seed: number) {
    this.s = seed || 1;
  }
  next(): number {
    let x = this.s;
    x ^= x << 13;
    x ^= x >>> 17;
    x ^= x << 5;
    return (this.s = x >>> 0);
  }
  below(n: number): number {
    return this.next() % n;
  }
  pick<T>(a: readonly T[]): T {
    return a[this.below(a.length)];
  }
}

const NOUNS = ["user", "session", "request", "config", "cache", "token", "record", "event", "queue", "worker",
  "buffer", "index", "schema", "route", "handler", "client", "server", "batch", "job", "metric", "account",
  "invoice", "order", "report", "message", "channel", "upload", "asset", "layer", "frame"] as const;
const VERBS = ["load", "parse", "build", "render", "fetch", "store", "update", "resolve", "flush", "encode",
  "decode", "merge", "apply", "check", "format", "collect", "emit", "retry", "compute", "split"] as const;
const WORDS = ["the", "a", "of", "to", "and", "in", "is", "for", "that", "with", "on", "as", "by", "this",
  "value", "returns", "when", "each", "first", "after", "before", "only", "every", "new", "list", "time",
  "result", "error", "state", "data", "file", "line", "name", "type", "key", "path", "case", "test", "call",
  "between", "without", "because", "which", "into", "while", "update", "change", "number", "small", "large"] as const;
const DIRS = ["src/server", "src/client", "src/core", "src/util", "src/api", "lib", "test", "scripts", "docs"] as const;
const EXTS = [".ts", ".ts", ".ts", ".js", ".rs", ".md", ".json"] as const;

const cap = (s: string) => s[0].toUpperCase() + s.slice(1);
const ident = (r: Rng) => r.pick(VERBS) + cap(r.pick(NOUNS));
const sentence = (r: Rng, n: number) => {
  const w = Array.from({ length: n }, () => r.pick(WORDS));
  return cap(w.join(" ")) + ".";
};
export function prose(r: Rng, chars: number): string {
  let s = "";
  while (s.length < chars) s += (s ? " " : "") + sentence(r, 6 + r.below(12));
  return s.slice(0, chars);
}
export function path(r: Rng): string {
  return `${r.pick(DIRS)}/${r.pick(NOUNS)}${r.below(3) ? "" : "-" + r.pick(NOUNS)}${r.pick(EXTS)}`;
}

/** Source-like text: imports, functions, comments (some TODO/FIXME), of `lines` lines. */
function source(r: Rng, lines: number): string {
  const out: string[] = [];
  for (let i = 0; i < 3 + r.below(5); i++) out.push(`import { ${ident(r)}, ${ident(r)} } from "./${r.pick(NOUNS)}";`);
  out.push("");
  while (out.length < lines) {
    const f = ident(r);
    const a = r.pick(NOUNS);
    if (r.below(3) === 0) out.push(`// ${r.below(4) === 0 ? (r.below(2) ? "TODO: " : "FIXME: ") : ""}${sentence(r, 5 + r.below(10))}`);
    out.push(`export ${r.below(4) === 0 ? "async " : ""}function ${f}(${a}: ${cap(a)}, opts = {}) {`);
    for (let j = 0; j < 2 + r.below(9); j++) {
      const k = r.below(5);
      if (k === 0) out.push(`  const ${r.pick(NOUNS)}${j} = ${ident(r)}(${a}, ${r.below(100)});`);
      else if (k === 1) out.push(`  if (!${a}.${r.pick(NOUNS)}) throw new Error("${sentence(r, 4)}");`);
      else if (k === 2) out.push(`  for (const item of ${a}.${r.pick(NOUNS)}s) ${ident(r)}(item);`);
      else if (k === 3) out.push(`  // ${sentence(r, 4 + r.below(8))}`);
      else out.push(`  ${a}.${r.pick(NOUNS)} = "${r.pick(WORDS)}-${r.below(1000)}";`);
    }
    out.push(`  return ${a};`, "}", "");
  }
  return out.slice(0, lines).join("\n") + "\n";
}

function fileText(p: string): string {
  const r = new Rng(hash("file " + p));
  // Most files an agent reads are a few hundred lines; some are long.
  const lines = 40 + r.below(r.below(5) === 0 ? 900 : 300);
  if (p.endsWith(".md")) return Array.from({ length: Math.ceil(lines / 4) }, (_, i) => (i % 6 === 0 ? `## ${cap(r.pick(NOUNS))} ${r.pick(NOUNS)}s\n\n` : "") + prose(r, 200 + r.below(300))).join("\n\n") + "\n";
  if (p.endsWith(".json")) return JSON.stringify(packageJson(r, p), null, 2) + "\n";
  return source(r, lines);
}

function packageJson(r: Rng, p: string) {
  const deps: Record<string, string> = {};
  for (let i = 0; i < 4 + r.below(10); i++) deps[`@acme/${r.pick(NOUNS)}-${r.pick(VERBS)}`] = `^${r.below(5)}.${r.below(20)}.${r.below(10)}`;
  const scripts: Record<string, string> = {};
  for (const s of ["build", "test", "lint", "clean", "copy-assets"]) scripts[s] = `${r.pick(["tsc -p .", "vitest --run", "eslint .", "rimraf dist", "cp -r assets dist"])}${r.below(3) ? "" : " && " + r.pick(["node scripts/check.mjs", "shx chmod +x dist/cli.js", "shx rm -rf build"])}`;
  return { name: `@acme/${p.split("/").slice(-2, -1)[0] ?? "pkg"}`, version: `${r.below(3)}.${r.below(30)}.${r.below(10)}`, type: "module", scripts, dependencies: deps };
}

function rg(r: Rng, pattern: string, files: number, perFile: number): string {
  const out: string[] = [];
  for (let i = 0; i < files; i++) {
    const p = path(r);
    for (let j = 0, n = 1 + r.below(perFile); j < n; j++) {
      const line = 1 + r.below(800);
      out.push(`${p}:${line}:  ${r.below(2) ? "const" : "return"} ${ident(r)}(${pattern.replace(/[^\w]/g, "")}${r.below(2) ? ", " + r.pick(NOUNS) : ""}); // ${sentence(r, 3 + r.below(6))}`);
    }
  }
  return out.join("\n") + "\n";
}

function bash(command: string, r: Rng): { output: string; exitCode: number } {
  let output: string;
  let exitCode = 0;
  if (command.startsWith("rg ")) output = rg(r, command.split(" ")[2] ?? "x", 25 + r.below(40), 12);
  else if (command.startsWith("git status")) output = Array.from({ length: 3 + r.below(12) }, () => `${r.pick([" M", "??", " D", "A "])} ${path(r)}`).join("\n") + "\n";
  else if (command.startsWith("git diff --stat")) {
    const n = 2 + r.below(10);
    output = Array.from({ length: n }, () => ` ${path(r).padEnd(40)} | ${String(1 + r.below(80)).padStart(3)} ${"+".repeat(1 + r.below(20))}${"-".repeat(r.below(10))}`).join("\n") + `\n ${n} files changed, ${r.below(400)} insertions(+), ${r.below(200)} deletions(-)\n`;
  } else if (command.startsWith("git log")) output = Array.from({ length: 8 }, () => `${r.next().toString(16).padStart(8, "0").slice(0, 7)} ${sentence(r, 4 + r.below(6)).slice(0, -1)}`).join("\n") + "\n";
  else if (command.startsWith("ls")) output = Array.from({ length: 5 + r.below(30) }, () => path(r).split("/").pop()).join("\n") + "\n";
  else if (command.startsWith("cat ") && command.endsWith(".json")) output = JSON.stringify(packageJson(r, command.slice(4)), null, 2) + "\n";
  else if (/\b(test|check|build)\b/.test(command)) {
    const lines: string[] = [];
    let failed = 0;
    for (let i = 0; i < 30 + r.below(120); i++) {
      const ok = r.below(25) !== 0;
      failed += ok ? 0 : 1;
      lines.push(`${ok ? "✓" : "✗"} ${path(r)} > ${ident(r)} ${sentence(r, 3 + r.below(5)).slice(0, -1)} (${r.below(300)} ms)`);
      if (!ok) lines.push(`    AssertionError: expected ${r.below(100)} to equal ${r.below(100)}`, `      at ${ident(r)} (${path(r)}:${r.below(500)}:${r.below(80)})`);
    }
    lines.push("", `Tests: ${failed} failed, ${lines.length - failed} passed`);
    output = lines.join("\n") + "\n";
    exitCode = failed ? 1 : 0;
  } else output = Array.from({ length: 5 + r.below(40) }, () => sentence(r, 4 + r.below(10))).join("\n") + "\n";
  return { output, exitCode };
}

function searchResults(r: Rng, n: number) {
  return {
    structuredContent: {
      results: Array.from({ length: n }, () => ({
        title: sentence(r, 4 + r.below(6)).slice(0, -1),
        url: `https://${r.pick(NOUNS)}.example.org/${r.pick(VERBS)}/${r.next().toString(36)}`,
        publishDate: `2026-${String(1 + r.below(12)).padStart(2, "0")}-${String(1 + r.below(28)).padStart(2, "0")}`,
        text: prose(r, 300 + r.below(1200)),
      })),
    },
  };
}

function rows(r: Rng, n: number) {
  const services = Array.from({ length: 8 }, () => `${r.pick(NOUNS)}-${r.pick(["api", "worker", "gateway", "cron"])}`);
  return {
    structuredContent: {
      rows: Array.from({ length: n }, (_, i) => ({
        ts: `2026-10-${String(1 + (i % 28)).padStart(2, "0")}T${String(r.below(24)).padStart(2, "0")}:${String(r.below(60)).padStart(2, "0")}:00Z`,
        service: r.pick(services),
        level: r.pick(["info", "info", "info", "warn", "error"]),
        status: r.pick([200, 200, 200, 201, 204, 304, 400, 404, 500, 503]),
        durationMs: r.below(5000) / 10,
        route: `/${r.pick(NOUNS)}s/${r.pick(["list", "get", "create", "update"])}`,
        userId: `u${r.below(400)}`,
        message: sentence(r, 4 + r.below(10)),
      })),
      meta: { rowCount: n, elapsedMs: r.below(900) },
    },
  };
}

/** Base64 of `bytes` pseudo-random bytes behind a PNG signature: an image's data URL. */
function image(r: Rng, bytes: number): string {
  const a = new Uint8Array(bytes);
  a.set([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  // Long runs of one byte, as in a rendered view, so it compresses like an image.
  for (let i = 8; i < bytes; ) {
    const v = r.below(256);
    for (let n = 1 + r.below(r.below(4) ? 4 : 64); n > 0 && i < bytes; n--) a[i++] = v;
  }
  return "data:image/png;base64," + Buffer.from(a).toString("base64");
}

/** The tool `name`'s answer to `args`. */
export function answer(name: string, args: Args): unknown {
  const r = new Rng(hash(name + JSON.stringify(args)));
  switch (name) {
    case "read": {
      const p = String(args.path);
      if (p.startsWith("/missing/")) throw new Error(`ENOENT: no such file or directory, open '${p}'`);
      let t = fileText(p);
      if (args.offset != null || args.limit != null) {
        const lines = t.split("\n");
        const from = Math.max(0, Number(args.offset ?? 1) - 1);
        t = lines.slice(from, from + Number(args.limit ?? lines.length)).join("\n");
      }
      return t;
    }
    case "bash":
      return bash(String(args.command), r);
    case "edit":
      return `Successfully replaced ${(args.edits as unknown[]).length} block(s) in ${args.path}.`;
    case "write":
      return `Successfully wrote ${String(args.content).length} bytes to ${args.path}`;
    case "web_search":
      return searchResults(r, Number(args.max_results ?? 5));
    case "query_rows":
      return rows(r, Number(args.limit ?? 500));
    case "render_view":
      return { path: `/tmp/render-${r.below(1000)}.png`, width: 1280, height: 720 };
    case "read_image":
      return image(r, 120_000 + r.below(80_000));
    default:
      throw new Error(`no stub tool ${name}`);
  }
}

/** The tool catalog a host lists to scripts (`ALL_TOOLS`): invented names and descriptions. */
export function catalog(): { name: string; description: string }[] {
  const r = new Rng(7);
  const out = ["read", "bash", "edit", "write", "web_search", "query_rows", "render_view", "read_image"].map((name) => ({ name, description: sentence(r, 8 + r.below(12)) }));
  for (const ns of ["docs", "tickets", "metrics", "storage", "deploy"])
    for (let i = 0; i < 6 + r.below(10); i++) out.push({ name: `${ns}__${r.pick(VERBS)}_${r.pick(NOUNS)}`, description: sentence(r, 8 + r.below(20)) });
  return out;
}

/**
 * Runs `body` (a codemode async function body) in node with the stub tools; returns the
 * answers it used, keyed `name + JSON.stringify(args)` as JSON text (an error as
 * `{"$error": message}`), and `ALL_TOOLS` under that key.
 */
export async function record(body: string): Promise<Record<string, string>> {
  const table: Record<string, string> = { ALL_TOOLS: JSON.stringify(catalog()) };
  const tools = new Proxy({}, {
    get: (_, name: string) => async (args: Args) => {
      const key = name + JSON.stringify(args);
      try {
        const v = answer(name, args);
        table[key] = JSON.stringify(v);
        return structuredClone(v);
      } catch (e) {
        table[key] = JSON.stringify({ $error: (e as Error).message });
        throw e;
      }
    },
  });
  const AsyncFunction = (async () => {}).constructor as new (...a: string[]) => (...v: unknown[]) => Promise<unknown>;
  const f = new AsyncFunction("tools", "text", "image", "ALL_TOOLS", body);
  await f(tools, () => {}, () => {}, catalog());
  return table;
}

/** A TypeScript body with its types removed (node's type stripping), for engines without TypeScript. */
export async function stripBody(body: string): Promise<string> {
  const { stripTypeScriptTypes } = await import("node:module");
  const head = "async function __body() {\n";
  const js = stripTypeScriptTypes(head + body + "\n}");
  return js.slice(head.length, js.lastIndexOf("}"));
}
