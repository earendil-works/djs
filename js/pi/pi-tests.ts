// Runs pi's own codemode tests (pi-private packages/codemode/test) against the djs adapter,
// without touching pi-private: copies the package's sources and tests into `--out` (default
// target/pi-codemode), swaps in two modules, and runs pi's installed vitest on the copy.
//
//   node js/pi/pi-tests.ts [--pi ~/Development/pi-private] [--out target/pi-codemode] [--no-run]
//                          [--no-lockdown] [--next [--dump-every-idle]] [--expected FILE]
//
// --no-lockdown runs pi's prelude with its `lockdown()` cut out, on frozen builtins (D2304):
// what pi would run on djs, and what the slim module needs (its builtins don't include the
// typed arrays the lockdown walks; D2411). The gate runs it on djs-slim.wasm.
// --next runs the adapter's `next` mode (D2650): tests/pi/prelude-next.ts instead of pi's
// prelude (tools as engine async host calls, no in-flight state in the prelude, frozen
// builtins instead of the lockdown). --dump-every-idle adds that at every idle point the
// session dumps itself (a plain Script, D2830) and goes on in a new instance resumed from
// the dump (D2651), loaded with the hooks (D2890, D2893).
// --expected FILE: the run passes when every failing test is listed in FILE (one full test
// name per line, `#` comments); listed tests that pass are reported as stale.
//
// The swapped modules:
// - src/runtime/vm.ts: `startVm` from js/pi/vm.ts with pi's own PRELUDE_SOURCE,
// - src/wasm.ts: `loadQuickJSWasm()` loads djs-wasm ($DJS_WASM, else the `wasm` profile build).
// Everything else (launchers, host, prelude, tests) is pi's, unchanged, but for one test
// fixture's workload (see below). The copy gets a
// node_modules of symlinks to pi-private's installed packages, and vitest's cache goes into
// the copy, so nothing is written under pi-private. Results: <out>/results.json (vitest's JSON
// reporter) and <out>/report.md (pass/fail per test, failure messages).

import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, relative, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../..");

function arg(name: string, fallback: string): string {
  const i = process.argv.indexOf(name);
  return i === -1 ? fallback : process.argv[i + 1];
}

const pi = resolve(arg("--pi", process.env.PI_PRIVATE ?? join(homedir(), "Development/pi-private")));
const out = resolve(arg("--out", join(repo, "target/pi-codemode")));
const wasm = resolve(process.env.DJS_WASM ?? join(repo, "target/wasm32-unknown-unknown/wasm/djs_wasm.wasm"));
const pkg = join(pi, "packages/codemode");
if (!existsSync(join(pkg, "src/runtime/vm.ts"))) throw new Error(`no pi codemode package at ${pkg}`);
if (!existsSync(wasm)) throw new Error(`no djs-wasm at ${wasm}: run scripts/build-wasm.sh or set DJS_WASM`);

const noLockdown = process.argv.includes("--no-lockdown");
const next = process.argv.includes("--next");
const dumpEveryIdle = process.argv.includes("--dump-every-idle");
if (dumpEveryIdle && !next) throw new Error("--dump-every-idle needs --next");
if (next && noLockdown) throw new Error("--next has no lockdown already");
const commit = spawnSync("git", ["-C", pi, "rev-parse", "HEAD"], { encoding: "utf8" }).stdout.trim();

rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
for (const entry of ["src", "test", "package.json", "tsconfig.build.json"]) {
  cpSync(join(pkg, entry), join(out, entry), { recursive: true });
}

// One test fixture is changed: the process-lifetime tests' workload. pi's is a slow
// built-in for QuickJS (a naive `indexOf` over 300,000 characters) that must still be
// running when the test cancels it 100-300 ms later; djs's `indexOf` finishes it in a few
// milliseconds, so the test passed or failed with the machine's load. An endless loop is
// still running at the cancel on any machine: the tests then check what they are about,
// prompt cancellation and prompt process exit, every time.
{
  const fixture = join(out, "test/fixtures/worker-process-exit.ts");
  const text = readFileSync(fixture, "utf8");
  const workload = "return 'a'.repeat(300000).indexOf('a'.repeat(10000) + 'b')";
  if (!text.includes(workload)) throw new Error(`${fixture}: pi's workload changed; update js/pi/pi-tests.ts`);
  writeFileSync(fixture, text.replace(workload, "while (true) {}"));
}

// node_modules: symlinks to pi-private's installed packages (resolution only; the cache
// below keeps vite from writing through them).
mkdirSync(join(out, "node_modules"));
for (const name of ["vitest", "quickjs-wasi"]) {
  symlinkSync(join(pi, "node_modules", name), join(out, "node_modules", name));
}

const adapter = relative(join(out, "src/runtime"), join(repo, "js/pi/vm.ts"));
const preludeNext = relative(join(out, "src/runtime"), join(repo, "tests/pi/prelude-next.ts"));
const engineModule = relative(join(out, "src"), join(repo, "js/src/sandbox.ts"));
const header = `// Generated by djs js/pi/pi-tests.ts from pi ${commit}: pi's module, swapped for djs.\n`;

writeFileSync(
  join(out, "src/runtime/vm.ts"),
  `${header}import { startVm as startDjsVm, type StartVmOptions as DjsOptions, type VmSession } from "${adapter}";
import { PRELUDE_SOURCE } from "./prelude-source.ts";

export type { VmSession };
export type StartVmOptions = Omit<DjsOptions, "prelude">;

${
    next
      ? `
import { PRELUDE_NEXT } from "${preludeNext}";

/** djs's prelude-next in the adapter's next mode (D2650). */
export function startVm(options: StartVmOptions): Promise<VmSession> {
\tvoid PRELUDE_SOURCE;
\treturn startDjsVm({ ...options, prelude: PRELUDE_NEXT, shape: "next"${dumpEveryIdle ? ", dumpEveryIdle: true" : ""} });
}
`
      : noLockdown
      ? `
/** pi's prelude without its lockdown (frozen builtins do that job, D2304). */
function withoutLockdown(prelude: string): string {
\tconst start = prelude.indexOf("\\t(function lockdown() {");
\tconst close = "\\n\\t})();";
\tconst end = prelude.indexOf(close, start);
\tif (start < 0 || end < 0) throw new Error("pi's prelude has no lockdown() to cut out");
\treturn prelude.slice(0, start) + prelude.slice(end + close.length);
}

export function startVm(options: StartVmOptions): Promise<VmSession> {
\treturn startDjsVm({ ...options, prelude: withoutLockdown(PRELUDE_SOURCE), frozenBuiltins: true });
}
`
      : `
export function startVm(options: StartVmOptions): Promise<VmSession> {
\treturn startDjsVm({ ...options, prelude: PRELUDE_SOURCE });
}
`
  }`,
);

writeFileSync(
  join(out, "src/wasm.ts"),
  `${header}import { loadModule } from "${engineModule}";

/** A compiled module; here djs-wasm. */
export type CodemodeWasmModule = object;

/** djs-wasm instead of quickjs-wasi: $DJS_WASM at generation time unless \`path\` is given. */
export function loadQuickJSWasm(path?: string): Promise<CodemodeWasmModule> {
\treturn loadModule(path ?? ${JSON.stringify(wasm)});
}
`,
);

writeFileSync(
  join(out, "vitest.config.ts"),
  `${header}import { defineConfig } from "vitest/config";

export default defineConfig({
\tcacheDir: ${JSON.stringify(join(out, ".vite"))},
\ttest: { globals: true, environment: "node", testTimeout: 30_000 },
\tresolve: { conditions: ["source"] },
\tssr: { resolve: { conditions: ["source"] } },
});
`,
);

if (process.argv.includes("--no-run")) process.exit(0);

const vitest = join(pi, "node_modules/vitest/vitest.mjs");
const run = spawnSync(
  process.execPath,
  [vitest, "run", "--root", out, "--reporter=dot", "--reporter=json", "--outputFile", join(out, "results.json")],
  { cwd: out, stdio: "inherit", env: { ...process.env, DJS_WASM: wasm } },
);

interface Assertion {
  fullName: string;
  status: string;
  failureMessages: string[];
}
const results = JSON.parse(readFileSync(join(out, "results.json"), "utf8")) as {
  numPassedTests: number;
  numFailedTests: number;
  numTotalTests: number;
  testResults: { name: string; message?: string; assertionResults: Assertion[] }[];
};
const lines = [
  `# pi codemode tests on djs`,
  ``,
  `pi ${commit}; djs-wasm ${relative(repo, wasm)}; ${results.numPassedTests} of ${results.numTotalTests} passed, ${results.numFailedTests} failed.`,
  ``,
];
for (const file of results.testResults) {
  const asserts = file.assertionResults;
  const passed = asserts.filter((a) => a.status === "passed").length;
  lines.push(`## ${relative(out, file.name)}: ${passed} / ${asserts.length}`, "");
  if (file.message) lines.push("```", file.message.trim().slice(0, 2000), "```", "");
  for (const a of asserts) {
    const mark = a.status === "passed" ? "pass" : a.status;
    lines.push(`- ${mark}: ${a.fullName}`);
    const why = (a.failureMessages[0] ?? "")
      .split("\n")
      .filter((line) => !/^\s+at /.test(line))
      .join("\n")
      .trim();
    if (why) lines.push("", "  ```", ...why.slice(0, 1500).split("\n").map((line) => `  ${line}`), "  ```", "");
  }
  lines.push("");
}
writeFileSync(join(out, "report.md"), lines.join("\n"));
console.log(`\n${results.numPassedTests} / ${results.numTotalTests} passed; report: ${relative(process.cwd(), join(out, "report.md"))}`);
const expectedFile = process.argv.includes("--expected") ? arg("--expected", "") : "";
if (expectedFile) {
  const expected = new Set(
    readFileSync(expectedFile, "utf8")
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line && !line.startsWith("#")),
  );
  const failed = results.testResults.flatMap((f) =>
    f.assertionResults.filter((a) => a.status === "failed").map((a) => a.fullName),
  );
  const fresh = failed.filter((name) => !expected.has(name));
  const stale = [...expected].filter((name) => !failed.includes(name));
  for (const name of stale) console.log(`stale (passes now): ${name}`);
  for (const name of fresh) console.log(`NEW FAILURE: ${name}`);
  // A file that fails to load reports no assertions: count those as failures too.
  const broken = results.testResults.filter((f) => f.assertionResults.length === 0 && f.message);
  for (const f of broken) console.log(`NEW FAILURE: ${relative(out, f.name)}: ${f.message?.split("\n")[0]}`);
  console.log(`pi tests: ${fresh.length + broken.length} new failures, ${stale.length} stale (${relative(repo, expectedFile)})`);
  process.exit(fresh.length + broken.length === 0 ? 0 : 1);
}
process.exit(run.status ?? 1);
