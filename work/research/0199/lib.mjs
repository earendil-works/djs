// Shared helpers of the 0199 scripts: running djs and node under a timeout, a pool.
import { execFile } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const here = path.dirname(fileURLToPath(import.meta.url));
export const root = path.resolve(here, "../../..");
export const DJS = process.env.DJS ?? path.join(root, "target/debug/djs");
const TIMEOUT = path.join(root, "scripts/timeout.sh");

/** Runs a command under scripts/timeout.sh; resolves {code, out} (stdout+stderr). */
export function run(cmd, args, { secs = 30, env } = {}) {
  return new Promise((resolve) => {
    execFile(
      TIMEOUT,
      [String(secs), cmd, ...args],
      { maxBuffer: 64 << 20, env: env ? { ...process.env, ...env } : process.env, cwd: here },
      (err, stdout, stderr) => resolve({ code: err ? (err.code ?? 1) : 0, stdout, stderr, out: stdout + stderr }),
    );
  });
}
export const djs = (args, opts) => run(DJS, args, opts);

/** Runs `jobs` (async thunks) with at most `n` at a time. */
export async function pool(jobs, n = 4) {
  const results = new Array(jobs.length);
  let next = 0;
  await Promise.all(
    Array.from({ length: n }, async () => {
      while (next < jobs.length) {
        const i = next++;
        results[i] = await jobs[i]();
      }
    }),
  );
  return results;
}

/** Output without stack lines (positions differ between a run and a dump). */
export const clean = (s) =>
  s
    .split("\n")
    .filter((l) => !/^\s+at /.test(l) && !l.startsWith("djs: --dump-after"))
    .join("\n")
    .replace(/\n+$/, "");
