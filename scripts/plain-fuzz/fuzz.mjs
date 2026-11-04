#!/usr/bin/env node
// fuzz.mjs: the plain-dump fuzzer (0208, D2940). Generated codemode-like programs
// (gen.mjs) run through the difftest's plain-dump variant (frozen builtins, D2836) with a
// chain of dumps starting at every idle point (`--plain-from K`), and every link's dump
// runs in node too (scripts/plain-node.mjs). Each failure gets a signature (what went
// wrong, numbers blanked); the report lists the distinct ones with their smallest program.
//
//   node scripts/plain-fuzz/fuzz.mjs [--seed S] [--cases N] [--time SECS] [--batch B]
//                                    [--jobs J] [--from-max K] [--dir DIR] [--no-node]
//   node scripts/plain-fuzz/fuzz.mjs check FILE...      one program (a case body) each
//   node scripts/plain-fuzz/fuzz.mjs reduce FILE --sig TEXT [--time SECS] [-o OUT]
//
// Seeds S, S+1, ... (default 1); stops after N programs (default 200) or SECS seconds
// (default 300), whichever comes first, at a batch boundary. The difftest binary is built
// first (`cargo build -p djs-difftest`), or taken from $DJS_DIFFTEST. Scratch goes to
// DIR (default work/scratch/0208/fuzz). `reduce` deletes lines of FILE while the program
// still fails with a signature containing TEXT (and still parses), and writes the result
// to OUT (default FILE.min.js). Exit status 1 if any program failed.
import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import vm from "node:vm";
import { generate } from "./gen.mjs";

const ROOT = path.resolve(new URL("../..", import.meta.url).pathname);
const argv = process.argv.slice(2);
const flag = (name, def) => {
  const i = argv.indexOf(name);
  if (i < 0) return def;
  const v = argv[i + 1];
  argv.splice(i, 2);
  return v;
};
const has = (name) => {
  const i = argv.indexOf(name);
  if (i >= 0) argv.splice(i, 1);
  return i >= 0;
};

const jobs = Number(flag("--jobs", 6));
const fromMax = Number(flag("--from-max", 40));
const noNode = has("--no-node");
const dir = path.resolve(flag("--dir", path.join(ROOT, "work/scratch/0208/fuzz")));

function difftestBin() {
  if (process.env.DJS_DIFFTEST) return path.resolve(process.env.DJS_DIFFTEST);
  const r = spawnSync("cargo", ["build", "-q", "-p", "djs-difftest", "--bin", "djs-difftest"], {
    cwd: ROOT,
    stdio: "inherit",
  });
  if (r.status !== 0) process.exit(2);
  return path.join(ROOT, "target/debug/djs-difftest");
}

// Runs a command, at most `jobs` at once; resolves to its stdout+stderr.
let running = 0;
const queue = [];
function run(cmd, args, opts = {}) {
  return new Promise((resolve) => {
    const start = () => {
      running++;
      const p = spawn(cmd, args, { cwd: opts.cwd ?? ROOT });
      let out = "";
      p.stdout.on("data", (d) => (out += d));
      p.stderr.on("data", (d) => (out += d));
      // A hang guard: the difftest's own limits (fuel, 120 s) come first.
      const t = setTimeout(() => p.kill("SIGKILL"), (opts.timeout ?? 600) * 1000);
      p.on("close", () => {
        clearTimeout(t);
        running--;
        if (queue.length) queue.shift()();
        resolve(out);
      });
    };
    running < jobs ? start() : queue.push(start);
  });
}

// What went wrong, with the numbers blanked, so that one bug is one signature.
const blank = (s) =>
  s
    .replace(/\d+/g, "N")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, 160);

// The difftest's stress mismatches: `--- NAME [LABEL]: REASON (- plain, + stressed)`,
// then the diff.
function parseDifftest(out) {
  const fails = [];
  const lines = out.split("\n");
  for (let i = 0; i < lines.length; i++) {
    const m = /^--- (\S+) \[djs [^\]]*\]: (.*) \(- plain, \+ stressed\)$/.exec(lines[i]);
    if (!m) continue;
    let sig = "wrong output";
    let detail = m[2];
    for (let j = i + 1; j < lines.length && !lines[j].startsWith("--- "); j++) {
      const e = /^\+=== (\w+)$/.exec(lines[j]);
      if (e && e[1] !== "return" && e[1] !== "completed") {
        detail = (lines[j + 1] ?? "").slice(1);
        sig = `${e[1]}: ${detail.split("\n")[0]}`;
        break;
      }
    }
    fails.push({ name: m[1], where: "djs", sig: "djs " + blank(sig), detail });
  }
  return fails;
}

function parseNode(out) {
  const fails = [];
  for (const l of out.split("\n")) {
    let m = /^(\S+) link (\d+): line \d+: expected (".*"), got (".*?")(?: (".*"))?$/.exec(l);
    if (m) {
      const got = JSON.parse(m[4]);
      const sig = m[5] ? `${got.slice(4)}: ${JSON.parse(m[5])}` : "wrong output";
      fails.push({ name: m[1], where: "node", sig: "node " + blank(sig), detail: l });
      continue;
    }
    m = /^(\S+) link (\d+): (.*)$/.exec(l);
    if (m) {
      fails.push({ name: m[1], where: "node", sig: "node " + blank(m[3]), detail: l });
      continue;
    }
    m = /^(\S+): refused, not in \S+: (.*)$/.exec(l);
    // The refused value's path (`<call 3><reaction 1>.promise...`) is left out.
    if (m) fails.push({ name: m[1], where: "refusal", sig: "refused " + blank(m[2].replace(/^idle point \d+: cannot dump: (?:<[^>]*>|[^\s:<])+: /, "")), detail: m[2] });
  }
  return fails;
}

// Runs the cases `names` (paths below corpus/) of the corpus in `work`: the chain from
// idle point 1, then from every later idle point a case has (at most --from-max).
// Returns the failures, each with the idle point its chain started at, and each case's
// idle points and plain transcript.
async function check(bin, work, names) {
  const corpus = path.join(work, "corpus");
  const empty = path.join(work, "no-refusals.txt");
  fs.writeFileSync(empty, "");
  fs.rmSync(path.join(work, "out"), { recursive: true, force: true });
  const unit = async (k, part, cases) => {
    const out = path.join(work, "out", `from-${k}${part}`);
    const args = ["--engine", "djs", "--corpus", corpus, "--no-diff", "-q", "-j", "1", "--plain-out", out, "--plain-from", String(k)];
    const text = await run(bin, [...args, ...(k === 1 ? ["--idle-points"] : []), ...cases]);
    const fails = parseDifftest(text).map((f) => ({ ...f, from: k }));
    if (!noNode && fs.existsSync(out)) {
      const n = await run("node", [path.join(ROOT, "scripts/plain-node.mjs"), out, "--refusals", empty]);
      fails.push(...parseNode(n).map((f) => ({ ...f, from: k })));
    }
    return { text, fails, out };
  };
  const per = Math.max(1, Math.ceil(names.length / jobs));
  const parts = [];
  for (let i = 0; i < names.length; i += per) parts.push(unit(1, `-${i}`, names.slice(i, i + per)));
  const firsts = await Promise.all(parts);
  const idle = new Map(names.map((n) => [n, 0]));
  const transcripts = new Map();
  const fails = [];
  for (const r of firsts) {
    fails.push(...r.fails);
    for (const m of r.text.matchAll(/^\s+(\d+)  (\S+)$/gm)) idle.set(m[2], Number(m[1]));
    for (const n of names) {
      const t = path.join(r.out, n, "transcript");
      if (fs.existsSync(t)) transcripts.set(n, fs.readFileSync(t, "utf8"));
    }
  }
  const max = Math.min(fromMax, Math.max(0, ...idle.values()));
  const later = [];
  for (let k = 2; k <= max; k++) {
    const cases = names.filter((n) => idle.get(n) >= k);
    if (cases.length) later.push(unit(k, "", cases));
  }
  for (const r of await Promise.all(later)) fails.push(...r.fails);
  return { idle, transcripts, fails };
}

// A program's file in the corpus layout the difftest wants: corpus/codemode/NAME.js
// (the area whose cases run with the builtin barrier armed, as pi's do) next to the
// prelude.
function corpusDir(work) {
  const c = path.join(work, "corpus");
  fs.rmSync(c, { recursive: true, force: true });
  fs.mkdirSync(path.join(c, "codemode"), { recursive: true });
  fs.copyFileSync(path.join(ROOT, "tests/corpus/prelude.js"), path.join(c, "prelude.js"));
  return c;
}

// The signatures of one program's failures (the reducer's test).
async function sigsOf(bin, work, text) {
  const c = corpusDir(work);
  fs.writeFileSync(path.join(c, "codemode/p.js"), text);
  const r = await check(bin, work, ["codemode/p.js"]);
  const sigs = new Set(r.fails.map((f) => f.sig));
  // The uninterrupted run's end too, for a bug that needs no dump.
  const m = /^=== (\w+)\n(.*)/m.exec(r.transcripts.get("codemode/p.js") ?? "");
  if (m) sigs.add(`plain ${m[1]}: ${blank(m[2])}`);
  return sigs;
}

// Parses as pi's case body (node's parser; djs's own errors would show as a crash).
function parses(text) {
  try {
    new vm.Script(`(async (tools, console) => {${text}\n})`);
    return true;
  } catch {
    return false;
  }
}

// The index of the bracket closing the one at `i` (strings and templates skipped
// roughly: good enough for generated code), or -1.
function matching(text, i) {
  const close = { "(": ")", "[": "]", "{": "}" };
  const stack = [];
  for (let j = i; j < text.length; j++) {
    const c = text[j];
    if (c === '"') {
      j = text.indexOf('"', j + 1);
      if (j < 0) return -1;
    } else if (close[c]) stack.push(close[c]);
    else if (c === ")" || c === "]" || c === "}") {
      if (stack.pop() !== c) return -1;
      if (!stack.length) return j;
    }
  }
  return -1;
}

async function reduce(bin, file, sig, out, deadline) {
  const work = path.join(dir, "reduce");
  let lines = fs.readFileSync(file, "utf8").split("\n");
  const fails = async (ls) => {
    const text = ls.join("\n");
    if (!parses(text)) return false;
    for (const s of await sigsOf(bin, work, text)) if (s.includes(sig)) return true;
    return false;
  };
  if (!(await fails(lines))) {
    console.error(`${file}: no failure with a signature containing ${JSON.stringify(sig)}`);
    process.exit(1);
  }
  // ddmin over lines: drop chunks, halving the chunk size when nothing goes.
  for (let n = Math.ceil(lines.length / 2); n >= 1; n = n === 1 ? 0 : Math.ceil(n / 2)) {
    let i = 0;
    while (i < lines.length && Date.now() < deadline) {
      const cand = [...lines.slice(0, i), ...lines.slice(i + n)];
      if (await fails(cand)) {
        lines = cand;
        fs.writeFileSync(out, lines.join("\n"));
      } else i += n;
    }
    if (n === 1) break;
  }
  // Simpler expressions: each bracketed group, outermost first, becomes `1` (or `[]`,
  // `{}`), as long as the program still fails the same way.
  let text = lines.join("\n");
  for (let i = 0; i < text.length && Date.now() < deadline; i++) {
    const open = text[i];
    if (!"([{".includes(open)) continue;
    const end = matching(text, i);
    if (end < 0) continue;
    for (const by of open === "(" ? ["1"] : open === "[" ? ["[]", "1"] : ["{}"]) {
      const cand = text.slice(0, i) + by + text.slice(end + 1);
      if (cand !== text && (await fails(cand.split("\n")))) {
        text = cand;
        fs.writeFileSync(out, text);
        break;
      }
    }
  }
  lines = text.split("\n");
  fs.writeFileSync(out, lines.join("\n").trimEnd() + "\n");
  console.log(`${out}: ${lines.length} lines`);
}

async function fuzz(bin) {
  const seed0 = Number(flag("--seed", 1));
  const cases = Number(flag("--cases", 200));
  const deadline = Date.now() + Number(flag("--time", 300)) * 1000;
  const batch = Number(flag("--batch", 60));
  const work = path.join(dir, "run");
  const found = new Map();
  let done = 0;
  let idlePoints = 0;
  const ends = new Map();
  for (let s = seed0; s < seed0 + cases && Date.now() < deadline; s += batch) {
    const c = corpusDir(work);
    const names = [];
    for (let k = s; k < Math.min(s + batch, seed0 + cases); k++) {
      fs.writeFileSync(path.join(c, `codemode/s${k}.js`), generate(k));
      names.push(`codemode/s${k}.js`);
    }
    const r = await check(bin, work, names);
    done += names.length;
    for (const n of names) {
      idlePoints += r.idle.get(n) ?? 0;
      // How the plain run ended (a throw with its message: a generator that writes
      // programs which fail by themselves tests less).
      const t = r.transcripts.get(n) ?? "=== (no transcript)\n";
      const m = /^=== (\w+)\n(.*)/m.exec(t);
      const end = !m ? "?" : m[1] === "throw" ? `throw (${blank(m[2])})` : m[1];
      ends.set(end, (ends.get(end) ?? 0) + 1);
    }
    for (const f of r.fails) {
      const seed = Number(/s(\d+)\.js/.exec(f.name)?.[1]);
      const size = generate(seed).length;
      const e = found.get(f.sig);
      if (!e) found.set(f.sig, { ...f, seed, size, count: 1, seeds: new Set([seed]) });
      else {
        e.seeds.add(seed);
        e.count++;
        if (size < e.size) Object.assign(e, { ...f, seed, size });
      }
    }
    process.stderr.write(`\r[${done} programs, ${idlePoints} idle points, ${found.size} signatures]`);
  }
  process.stderr.write("\n");
  console.log(`plain-fuzz: seeds ${seed0}..${seed0 + done - 1}, ${done} programs, ${idlePoints} idle points`);
  console.log(`plain runs end: ${[...ends].map(([e, n]) => `${n} ${e}`).join(", ")}`);
  for (const [sig, e] of [...found].sort((a, b) => b[1].seeds.size - a[1].seeds.size)) {
    console.log(`${String(e.seeds.size).padStart(4)} programs  ${sig}\n        smallest: seed ${e.seed} (${e.size} bytes), chain from idle point ${e.from}`);
  }
  process.exit(found.size ? 1 : 0);
}

const bin = difftestBin();
if (argv[0] === "check") {
  const work = path.join(dir, "check");
  const c = corpusDir(work);
  const names = argv.slice(1).map((f, i) => {
    fs.copyFileSync(f, path.join(c, `codemode/c${i}.js`));
    return `codemode/c${i}.js`;
  });
  const r = await check(bin, work, names);
  for (const f of r.fails) console.log(`${argv[1 + Number(/c(\d+)/.exec(f.name)[1])]} (chain from ${f.from}): ${f.sig}\n    ${f.detail}`);
  console.log(`check: ${names.length} programs, ${[...r.idle.values()].reduce((a, b) => a + b, 0)} idle points, ${r.fails.length} failures`);
  process.exit(r.fails.length ? 1 : 0);
} else if (argv[0] === "reduce") {
  const sig = flag("--sig", "");
  const out = flag("-o", argv[1].replace(/\.js$/, "") + ".min.js");
  const deadline = Date.now() + Number(flag("--time", 600)) * 1000;
  await reduce(bin, argv[1], sig, out, deadline);
} else if (argv[0] === "gen") {
  process.stdout.write(generate(Number(argv[1])));
} else {
  await fuzz(bin);
}
