// Classifies every `await` of codemode scripts by what a portable snapshot would have to
// describe if the script were suspended there (issue 0141, docs/portable-snapshots.md).
//
// The classification is source-level (an ECMAScript AST, no compiler): walking from the
// `await` up to its function, it collects what the spec's evaluation order holds while
// the await waits:
// - expression operands evaluated before the await and still needed after it (the callee
//   and `this` of a call whose argument awaits, earlier arguments, the left side of a
//   binary operator, the string so far of a template literal, an array or object
//   literal under construction, the object of `o[await k]` or `o.p = await v`, the old
//   value of `x += await v`);
// - statement state: the iterator of an enclosing for-of / for-in body, the
//   discriminant of a switch, the pending completion of a finally block.
//
// Usage:
//   node await-sites.mjs --jsonl extract.jsonl [--acorn PATH]   # aggregate counts only
//   node await-sites.mjs --files a.js b.js ... [--sites OUT.json] # per-site table too
//
// Private data (work/scratch/0130/extract.jsonl) is read, never printed: the output is
// counts. `--sites` writes file:line:col -> category for the harvest corpus (redacted).

import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";

const argv = process.argv.slice(2);
const opt = (name) => {
  const i = argv.indexOf(name);
  return i >= 0 ? argv[i + 1] : undefined;
};
const require = createRequire(import.meta.url);
const acorn = require(opt("--acorn") ?? "acorn");

const FUNC = new Set(["FunctionDeclaration", "FunctionExpression", "ArrowFunctionExpression"]);

function parse(code) {
  return acorn.parse(code, {
    ecmaVersion: "latest",
    sourceType: "script",
    allowReturnOutsideFunction: true,
    allowAwaitOutsideFunction: true,
    locations: true,
  });
}

function setParents(ast) {
  const stack = [ast];
  while (stack.length) {
    const n = stack.pop();
    for (const k of Object.keys(n)) {
      if (k === "__parent" || k === "loc") continue;
      const v = n[k];
      if (Array.isArray(v)) {
        v.forEach((c, i) => {
          if (c && typeof c.type === "string") {
            c.__parent = n;
            c.__key = k;
            c.__index = i;
            stack.push(c);
          }
        });
      } else if (v && typeof v.type === "string") {
        v.__parent = n;
        v.__key = k;
        stack.push(v);
      }
    }
  }
}

// What the spec holds at an await, from the await up to its function.
function classify(aw) {
  const held = [];
  const statement = [];
  let fn = null;
  let c = aw;
  for (let p = aw.__parent; p; c = p, p = p.__parent) {
    if (FUNC.has(p.type)) {
      fn = p;
      break;
    }
    const key = c.__key;
    const idx = c.__index;
    switch (p.type) {
      case "CallExpression":
      case "NewExpression":
        if (key === "arguments") {
          held.push(p.type === "NewExpression" ? "new.callee" : "call.callee");
          const callee = p.callee.type === "ChainExpression" ? p.callee.expression : p.callee;
          if (p.type === "CallExpression" && callee.type === "MemberExpression") held.push("call.this");
          for (let i = 0; i < idx; i++) held.push("call.arg");
        }
        break;
      case "MemberExpression":
        if (key === "property" && p.computed) held.push("member.object");
        break;
      case "BinaryExpression":
        if (key === "right") held.push("binary.left");
        break;
      case "AssignmentExpression":
        if (key === "right") {
          const l = p.left;
          if (l.type === "MemberExpression") {
            held.push("assign.object");
            if (l.computed) held.push("assign.key");
          }
          if (p.operator !== "=" && !["||=", "&&=", "??="].includes(p.operator)) held.push("assign.old");
        }
        break;
      case "ArrayExpression":
        held.push("array.literal");
        break;
      case "ObjectExpression":
        held.push("object.literal");
        break;
      case "Property":
        if (key === "value" && p.computed) held.push("object.key");
        break;
      case "TemplateLiteral":
        held.push("template.prefix");
        break;
      case "TaggedTemplateExpression":
        held.push("tagged");
        break;
      case "SpreadElement":
      case "ConditionalExpression":
      case "LogicalExpression":
      case "SequenceExpression":
      case "UnaryExpression":
      case "UpdateExpression":
      case "AwaitExpression":
      case "ChainExpression":
      case "ParenthesizedExpression":
      case "VariableDeclarator":
      case "VariableDeclaration":
      case "ExpressionStatement":
      case "ReturnStatement":
      case "ThrowStatement":
      case "IfStatement":
      case "BlockStatement":
      case "WhileStatement":
      case "DoWhileStatement":
      case "ForStatement":
      case "LabeledStatement":
      case "SwitchCase":
      case "CatchClause":
      case "Program":
        break;
      case "ForOfStatement":
      case "ForInStatement":
        if (key === "body") statement.push(p.type === "ForOfStatement" ? (p.await ? "loop.forawait" : "loop.forof") : "loop.forin");
        else if (key === "left") held.push("pattern");
        break;
      case "SwitchStatement":
        if (key === "cases") statement.push("switch");
        break;
      case "TryStatement":
        if (key === "finalizer") statement.push("finally");
        break;
      case "AssignmentPattern":
      case "ArrayPattern":
      case "ObjectPattern":
        held.push("pattern");
        break;
      default:
        held.push("other:" + p.type);
    }
  }
  let cat;
  const stmtOther = statement.filter((s) => !s.startsWith("loop."));
  if (held.length === 0 && statement.length === 0) cat = "A";
  else if (held.length === 0 && stmtOther.length === 0) cat = "B";
  else if (stmtOther.length === 0) cat = "C";
  else cat = "D";
  return { cat, held, statement, inBody: fn === null };
}

function sitesOf(code) {
  const ast = parse(code);
  setParents(ast);
  const out = [];
  const stack = [ast];
  while (stack.length) {
    const n = stack.pop();
    if (n.type === "AwaitExpression" || (n.type === "ForOfStatement" && n.await)) {
      const r = classify(n);
      r.line = n.loc.start.line;
      r.col = n.loc.start.column + 1;
      r.type = n.type;
      out.push(r);
    }
    for (const k of Object.keys(n)) {
      if (k === "__parent" || k === "loc") continue;
      const v = n[k];
      if (Array.isArray(v)) for (const c of v) c && typeof c.type === "string" && stack.push(c);
      else if (v && typeof v.type === "string") stack.push(v);
    }
  }
  return out;
}

const totals = { scripts: 0, parseErrors: 0, awaits: 0, cat: {}, roles: {}, roleSets: {}, stmt: {}, depth: {}, scriptWorst: {}, nested: 0 };
const bump = (o, k, n = 1) => (o[k] = (o[k] ?? 0) + n);
const order = ["none", "A", "B", "C", "D"];
const siteTable = {};

function account(name, code) {
  totals.scripts++;
  let sites;
  try {
    sites = sitesOf(code);
  } catch {
    totals.parseErrors++;
    return;
  }
  let worst = "none";
  for (const s of sites) {
    totals.awaits++;
    bump(totals.cat, s.cat);
    if (!s.inBody) totals.nested++;
    for (const r of new Set(s.held)) bump(totals.roles, r);
    if (s.held.length) bump(totals.roleSets, [...new Set(s.held)].sort().join(" + "));
    for (const r of new Set(s.statement)) bump(totals.stmt, r);
    bump(totals.depth, Math.min(s.held.length, 6));
    if (order.indexOf(s.cat) > order.indexOf(worst)) worst = s.cat;
    if (name) siteTable[`${name}:${s.line}:${s.col}`] = { cat: s.cat, held: s.held, statement: s.statement };
  }
  bump(totals.scriptWorst, worst);
}

if (opt("--jsonl")) {
  const seen = new Set();
  for (const line of readFileSync(opt("--jsonl"), "utf8").split("\n")) {
    if (!line) continue;
    const { code } = JSON.parse(line);
    if (seen.has(code)) continue;
    seen.add(code);
    account(null, code);
  }
} else {
  const i = argv.indexOf("--files");
  for (const f of argv.slice(i + 1).filter((a) => !a.startsWith("--") && a !== opt("--sites") && a !== opt("--acorn"))) {
    account(f, readFileSync(f, "utf8"));
  }
}
if (opt("--sites")) writeFileSync(opt("--sites"), JSON.stringify(siteTable, null, 1));
const pct = (n, d) => `${n} (${((100 * n) / d).toFixed(1)}%)`;
console.log(`scripts ${totals.scripts}, parse errors ${totals.parseErrors}, awaits ${totals.awaits} (${totals.nested} in nested functions)`);
console.log("await sites by category:");
for (const c of ["A", "B", "C", "D"]) console.log(`  ${c}: ${pct(totals.cat[c] ?? 0, totals.awaits)}`);
console.log("scripts by their worst await site:");
for (const c of order) console.log(`  ${c}: ${pct(totals.scriptWorst[c] ?? 0, totals.scripts - totals.parseErrors)}`);
console.log("expression roles (await sites holding at least one):");
for (const [k, v] of Object.entries(totals.roles).sort((a, b) => b[1] - a[1])) console.log(`  ${k}: ${pct(v, totals.awaits)}`);
console.log("held role sets (await sites):");
for (const [k, v] of Object.entries(totals.roleSets).sort((a, b) => b[1] - a[1]).slice(0, 12)) console.log(`  ${k}: ${pct(v, totals.awaits)}`);
console.log("statement state:");
for (const [k, v] of Object.entries(totals.stmt).sort((a, b) => b[1] - a[1])) console.log(`  ${k}: ${pct(v, totals.awaits)}`);
console.log("held expression operands per await (6 = 6 or more):");
for (const [k, v] of Object.entries(totals.depth)) console.log(`  ${k}: ${pct(v, totals.awaits)}`);
