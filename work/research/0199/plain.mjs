// 0199 prototype: turn a format 2 dump (djs's exporter) into a plain-JavaScript dump.
//
// A plain dump is a Script that needs nothing but the host's functions: no loader, no
// privileged primitives. Every suspended activation becomes a *continuation* K, a function
// whose body is the rest of the original function f from its site; K is written where f
// lives (its factory, or inside another continuation when f's scope is a live frame), and
// K is itself the factory of its frame's scopes: the variables the frame shares with
// closures are K's own declarations and the closures are made inside K, so a variable in
// its TDZ is exactly that (its declaration is in the rest of K's code).
//
// Origin tracking: K carries `$djs = {f, at, segs, scaf}`: the original function (or its
// text), the site K stands for, and how K's text maps back to f's. A dump of a run that
// resumed continuations is first mapped back (normalize.mjs) to activations of the
// original functions at original sites, so K is always generated from the original text
// and a dump's size stays bounded over any number of re-dumps.
//
// Usage: toPlain(dumpText) -> { text, stats }; throws Refuse(reason).
import * as acorn from "acorn";
import { freeVars } from "./freevars.mjs";

export class Refuse extends Error {}
export const refuse = (m) => {
  throw new Refuse(m);
};

// ---- AST helpers --------------------------------------------------------------------------
export function children(n) {
  const out = [];
  for (const k of Object.keys(n)) {
    if (k === "type" || k === "start" || k === "end" || k === "range" || k === "loc") continue;
    const v = n[k];
    if (Array.isArray(v)) for (const c of v) c && typeof c.type === "string" && out.push(c);
    else if (v && typeof v.type === "string") out.push(v);
  }
  return out.sort((a, b) => a.start - b.start);
}
export const isFn = (n) =>
  n.type === "FunctionDeclaration" || n.type === "FunctionExpression" || n.type === "ArrowFunctionExpression";
const isLoop = (n) => /^(For|ForIn|ForOf|While|DoWhile)Statement$/.test(n.type);

export function parseScript(text) {
  return acorn.parse(text, { ecmaVersion: "latest", sourceType: "script", allowHashBang: false });
}

/** A function's text parsed on its own: {fn, node, kind}; offsets relative to the text. */
export function parseFunctionText(text) {
  // A method, getter or setter (`async m() {}`, `*g() {}`, `get x() {}`) or a function.
  let isMethod = !/^(async\s+)?function\b/.test(text) && !/^(async\s*)?(\(|[\w$]+\s*=>)/.test(text);
  if (/^(async\s+)?[\w$]+\s*=>/.test(text) || /^async\s*\(/.test(text) || text.startsWith("(")) isMethod = false;
  if (isMethod) {
    const ast = parseScript("({" + text + "})");
    const prop = ast.body[0].expression.properties[0];
    shift(prop, -2);
    return { fn: prop.value, prop, kind: prop.kind === "get" || prop.kind === "set" ? prop.kind : "method" };
  }
  const ast = parseScript("(" + text + ")");
  const fn = ast.body[0].expression;
  shift(fn, -1);
  return { fn, kind: fn.type === "ArrowFunctionExpression" ? "arrow" : "function" };
}
function shift(n, d) {
  n.start += d;
  n.end += d;
  for (const c of children(n)) shift(c, d);
}

// Sites of `fn` in source order, not inside nested functions (djs numbers them by the
// position of the keyword, per kind).
export function sites(fn) {
  const out = { await: [], yield: [], forAwait: [] };
  (function visit(n) {
    if (n !== fn && isFn(n)) return;
    if (n.type === "AwaitExpression") out.await.push(n);
    if (n.type === "YieldExpression" && !n.delegate) out.yield.push(n);
    if (n.type === "ForOfStatement" && n.await) out.forAwait.push(n);
    for (const c of children(n)) visit(c);
  })(fn);
  for (const k of Object.keys(out)) out[k].sort((a, b) => a.start - b.start);
  out["for-await.next"] = out.forAwait;
  return out;
}

export function pathTo(root, target) {
  const path = [];
  (function visit(n) {
    path.push(n);
    if (n === target) return true;
    for (const c of children(n)) if (c.start <= target.start && target.end <= c.end && visit(c)) return true;
    path.pop();
    return false;
  })(root);
  return path;
}

export function patNames(p, out = []) {
  if (!p) return out;
  if (p.type === "Identifier") out.push(p.name);
  else if (p.type === "ObjectPattern") p.properties.forEach((q) => patNames(q.type === "RestElement" ? q.argument : q.value, out));
  else if (p.type === "ArrayPattern") p.elements.forEach((e) => patNames(e, out));
  else if (p.type === "RestElement") patNames(p.argument, out);
  else if (p.type === "AssignmentPattern") patNames(p.left, out);
  return out;
}

/** Names a statement declares in its statement list: [{name, kind}]. */
function lexDecls(s) {
  if (s.type === "VariableDeclaration" && s.kind !== "var")
    return s.declarations.flatMap((d) => patNames(d.id).map((name) => ({ name, kind: s.kind })));
  if (s.type === "ClassDeclaration") return [{ name: s.id.name, kind: "let" }];
  if (s.type === "FunctionDeclaration") return [{ name: s.id.name, kind: "function" }];
  return [];
}
/** var names anywhere in `n`, not in nested functions. */
function varNames(n, out = []) {
  if (isFn(n)) return out;
  if (n.type === "VariableDeclaration" && n.kind === "var") n.declarations.forEach((d) => patNames(d.id, out));
  for (const c of children(n)) varNames(c, out);
  return out;
}

// ---- text pieces ----------------------------------------------------------------------------
// Generated code is a tree of strings and verbatim spans of the origin text {v: [s, e]};
// flattening gives the text and the segments [kStart, fStart, len] (origin tracking).
export function flatten(parts, ftext, markRecs = () => []) {
  let text = "";
  let helperAt = -1;
  const segs = [];
  (function walk(p) {
    if (p == null || p === false) return;
    if (typeof p === "string") text += p;
    else if (Array.isArray(p)) p.forEach(walk);
    else if (p.v) {
      segs.push(text.length, p.v[0], p.v[1] - p.v[0]);
      text += ftext.slice(p.v[0], p.v[1]);
    } else if (p.mark) p.mark(text.length);
    else if (p.helpers) {
      helperAt = text.length;
    }
  })(parts);
  // K's captured helpers go where the marker is; offsets after it move.
  if (helperAt >= 0) {
    const names = ["R", "G", "C", "D", "A", "L", "N", "Q"].filter((h) => text.includes(`$djs_${h}(`));
    const decl = names.length ? `const ${names.map((h) => `$djs_${h} = $dump.${h}`).join(", ")};\n` : "";
    text = text.slice(0, helperAt) + decl + text.slice(helperAt);
    for (let i = 0; i < segs.length; i += 3) if (segs[i] >= helperAt) segs[i] += decl.length;
    for (const r of markRecs()) if (r[2] >= helperAt) r[2] += decl.length;
  }
  return { text, segs };
}
const join = (arr, sep) => arr.flatMap((x, i) => (i ? [sep, x] : [x]));

// ---- the continuation ---------------------------------------------------------------------
// gen({ftext, fn, kind, site, slot, cellOf, blockContent, go}) -> {parts, scaf, used}
//   slot(name) -> D expression text of the slot `name` (or undefined)
//   cellOf(name) -> {init: text|null} when the slot is a variable a merged factory shares
//   blockContent(names, kblock) -> parts: the merged factories' code for the K block that
//     declares `names` (called once per block, outermost first), plus deferred bindings
//   go -> text to run right before the partial statement (the dump's callback), or ""
// K is written as a function of f's kind with no parameters; its first part (the
// "prefix") declares the live bindings, makes the shared scopes' closures and reaches the
// site's statement, which is rewritten with the evaluated values in place.
export function gen(ctx) {
  const { ftext, fn, site } = ctx;
  const P = (n) => ({ v: [n.start, n.end] });
  const used = new Set(); // slot names consumed
  const scaf = []; // [kind, fOffset, mark] scaffold constructs: their K offset is set by flatten
  const markScaf = (kind, fOff) => {
    const rec = [kind, fOff, -1];
    scaf.push(rec);
    return { mark: (k) => (rec[2] = k) };
  };
  const path = pathTo(fn.body, site);
  // Origin of K's own temporaries in the site's rewritten statement: K slot construct@k
  // stands for f's slot fName (null: K's helper, dropped).
  const cmap = [];
  const cm = (fName, kConstruct) => ({ mark: (k) => cmap.push([fName, kConstruct, k]) });
  const ign = (...ks) => ks.map((kConstruct) => ({ mark: (k) => cmap.push([null, kConstruct, k]) }));
  const hasSlot = (name) => ctx.slot(name) !== undefined;
  const take = (name) => {
    const v = ctx.slot(name);
    if (v === undefined) refuse(`missing slot ${name}`);
    used.add(name);
    return v;
  };
  // Temporaries of the site's expression: evaluated once, in the prefix (the dump's
  // values are reachable only while it loads), as consts right before the statement.
  const temps = [];
  const tempNames = []; // origin: which f temporary each K const (or literal) stands for
  let tempCount = 0;
  const tmp = (construct, off) => {
    const v = take(`${construct}@${off}`);
    if (/^(undefined|-?\d+(\.\d+)?|"[^"\\]*"|true|false|null)$/.test(v)) {
      tempNames.push([`${construct}@${off}`, 0, v]);
      return v;
    }
    temps.push(v);
    const name = `$djs_t${tempCount++}`;
    tempNames.push([`${construct}@${off}`, 1, name]);
    return name;
  };
  const tmpOpt = (construct, off) => (hasSlot(`${construct}@${off}`) ? tmp(construct, off) : null);
  const inline = (construct, off) => take(`${construct}@${off}`);
  // the array of a spread call's (or new's) arguments so far: named by the `(`
  const tmpAcc = (construct, from, n) => {
    const open = ftext.indexOf("(", from);
    const name = `${construct}@${open}`;
    if (!hasSlot(name)) refuse(`missing slot ${name}`);
    return { name, v: tmp(construct, open) };
  };
  const flushTemps = () => {
    if (!temps.length) return "";
    const base = tempCount - temps.length;
    const t = `const ${temps.map((v, k) => `$djs_t${base + k} = ${v}`).join(", ")};\n`;
    temps.length = 0;
    return t;
  };
  let usesArguments = false;
  const isArrow = fn.type === "ArrowFunctionExpression";

  // The rest of the code `n`: verbatim, with `arguments` renamed (K has no arguments of
  // f's: they are a slot).
  function rest(n) {
    const repl = [];
    if (!isArrow)
      (function visit(m) {
        if (m !== n && isFn(m) && m.type !== "ArrowFunctionExpression") return;
        if (m.type === "Identifier" && m.name === "arguments") repl.push(m);
        if (m.type === "MetaProperty" || m.type === "Super") refuse("new.target or super in a continuation");
        for (const c of children(m)) visit(c);
      })(n);
    if (!repl.length) return P(n);
    usesArguments = true;
    const out = [];
    let at = n.start;
    for (const m of repl) {
      out.push({ v: [at, m.start] }, "$djs_arguments");
      at = m.end;
    }
    out.push({ v: [at, n.end] });
    return out;
  }

  // ---- bindings ------------------------------------------------------------------------
  // Which slot names a binding `name` declared by scope node `scopeNode` has: `name`, or
  // `name@S` when shadowed (S: the declaring scope's offset; djs names the scope by...).
  function bindingSlot(name, scopeNode) {
    for (const k of [`${name}@${scopeNode.start}`, name]) if (hasSlot(k)) return k;
    // shadowed names: any name@S whose S lies in the scope node
    return null;
  }
  const kindOf = (k) => (k === "const" ? "const" : k === "var" || k === "param" || k === "fnvar" ? "var" : "let");

  // Declarations of a K block: decls [{name, kind, live}] in scope there; returns parts:
  // the live bindings' declarations and the code the dump puts in this block.
  let blockIndex = 0;
  let goDone = false;
  function declare(decls, scopeNode) {
    const early = [];
    const late = [];
    const after = [];
    const info = [];
    for (const { name, kind, live } of decls) {
      const sname = bindingSlot(name, scopeNode);
      info.push({ name, sname, live });
      if (!live || sname === null) continue; // declared by the rest of the code
      used.add(sname);
      const cell = ctx.cellOf(sname);
      let value;
      if (cell) {
        if (cell.init === null) continue; // in its TDZ: the rest declares it
        value = cell.init;
      } else {
        value = ctx.slot(sname);
        if (value === "$dump.TDZ") continue;
      }
      const target = name === "arguments" ? "$djs_arguments" : name;
      if (name === "arguments") usesArguments = true;
      const text = `${kindOf(kind)} ${target} = ${value};`;
      if (ctx.afterGo?.(sname)) after.push(text);
      else (/\$dump\.s\d+/.test(value) ? late : early).push(text);
    }
    const content = ctx.blockContent(blockIndex++, info) ?? [];
    // Bindings whose values the dump makes only after this frame's closures exist: the
    // dump's callback runs first.
    const go = after.length && !goDone && ctx.go ? ((goDone = true), [ctx.go]) : [];
    return [...early, ...content, ...late, ...go, ...after].join("\n");
  }

  // The bindings a function body declares: params, vars, lexical and functions.
  function functionDecls(j) {
    const out = [];
    for (const p of fn.params) patNames(p).forEach((name) => out.push({ name, kind: "param", live: true }));
    if (!isArrow && hasSlot("arguments")) out.push({ name: "arguments", kind: "var", live: true });
    if (fn.body.type === "BlockStatement") {
      varNames(fn.body).forEach((name) => out.push({ name, kind: "var", live: true }));
      fn.body.body.forEach((s, k) => {
        for (const d of lexDecls(s)) out.push({ ...d, kind: d.kind === "function" ? "fnvar" : d.kind, live: k < j || d.kind === "function" });
      });
    }
    const seen = new Set();
    return out.filter((d) => !seen.has(d.name) && seen.add(d.name));
  }

  // ---- expressions on the path, from the site up -----------------------------------------
  function expr(i, superThis) {
    const n = path[i];
    const next = path[i + 1];
    if (n === site) {
      // (marked: K's own await of the site is the original site)
      if (n.type === "AwaitExpression") return ["(", markScaf("site", n.start), "await ", ctx.awaiting, ")"];
      return ["(", markScaf("site", n.start), "yield)"]; // a yield's operand was yielded before the dump
    }
    switch (n.type) {
      case "AwaitExpression":
        return ["(await ", expr(i + 1), ")"];
      case "ParenthesizedExpression":
        return expr(i + 1);
      case "ChainExpression":
        return expr(i + 1);
      case "CallExpression": {
        if (next === n.callee && n.callee.type === "MemberExpression" && n.callee.object.type === "Super") {
          // super[key](args) with the key awaited: the method read from the saved base with
          // the saved this, then called with it
          const st = tmp("super.this", n.callee.start);
          return ["$djs_R(", expr(i + 1, st), ", ", st, ", [", join(n.arguments.map(rest), ", "), "])"];
        }
        if (next === n.callee) return [expr(i + 1), n.optional ? "?.(" : "(", join(n.arguments.map(rest), ", "), ")"];
        if (n.callee.type === "Super" || n.callee.type === "Import") refuse("super/import call around a site");
        const callee = tmp("call.callee", n.start);
        const self = tmp("call.this", n.start);
        const site = n.arguments.indexOf(next);
        const spreadAt = n.arguments.findIndex((a) => a.type === "SpreadElement");
        const args = [];
        if (spreadAt >= 0 && spreadAt < site) {
          // the argument list so far is one array: the rest is appended to it
          const acc = tmpAcc("call.args", n.callee.end, n);
          const more = n.arguments.slice(site + 1).map(rest);
          return [ign("call.callee", "call.this"), "$djs_R(", cm(`call.callee@${n.start}`, "call.arg"), callee, ", ", cm(`call.this@${n.start}`, "call.arg"), self, ", ", ign("call.callee", "call.this"), "$djs_Q(", cm(acc.name, "call.arg"), acc.v, ", ", join([expr(i + 1), ...more], ", "), "))"];
        }
        n.arguments.forEach((a, k) => {
          const c = cm(`call.arg@${a.start}`, "array.element");
          if (k < site) args.push([c, tmp("call.arg", a.start)]);
          else if (k === site) args.push([c, expr(i + 1)]);
          else args.push([c, rest(a)]);
        });
        // The call with the callee and this value evaluated before the site.
        return [
          ign("call.callee", "call.this"),
          "$djs_R(",
          cm(`call.callee@${n.start}`, "call.arg"),
          callee,
          ", ",
          cm(`call.this@${n.start}`, "call.arg"),
          self,
          ", ",
          ign("call.arg"),
          "[",
          join(args, ", "),
          "])",
        ];
      }
      case "NewExpression": {
        if (next === n.callee) refuse("site in a new's callee");
        const callee = tmp("new.callee", n.start);
        const site = n.arguments.indexOf(next);
        if (n.arguments.some((a) => a.type === "SpreadElement")) {
          const acc = tmpAcc("new.args", n.callee.end, n);
          const more = n.arguments.slice(site + 1).map(rest);
          return [ign("call.callee", "call.this"), "$djs_C(", cm(`new.callee@${n.start}`, "call.arg"), callee, ", ", ign("call.callee", "call.this"), "$djs_Q(", cm(acc.name, "call.arg"), acc.v, ", ", join([expr(i + 1), ...more], ", "), "))"];
        }
        const args = n.arguments.map((a, k) => [cm(`new.arg@${a.start}`, "array.element"), k < site ? tmp("new.arg", a.start) : k === site ? expr(i + 1) : rest(a)]);
        return [ign("call.callee", "call.this"), "$djs_C(", cm(`new.callee@${n.start}`, "call.arg"), callee, ", ", ign("call.arg"), "[", join(args, ", "), "])"];
      }
      case "BinaryExpression":
        if (next === n.left) return ["(", expr(i + 1), ` ${n.operator} `, rest(n.right), ")"];
        return ["(", cm(`binary.left@${n.start}`, "binary.left"), tmp("binary.left", n.start), ` ${n.operator} `, expr(i + 1), ")"];
      case "LogicalExpression":
        if (next === n.left) return ["(", expr(i + 1), ` ${n.operator} `, rest(n.right), ")"];
        return expr(i + 1); // the left side decided to evaluate the right
      case "ConditionalExpression":
        if (next === n.test) return ["(", expr(i + 1), " ? ", rest(n.consequent), " : ", rest(n.alternate), ")"];
        return expr(i + 1);
      case "UnaryExpression":
        return ["(", n.operator, n.operator.length > 1 ? " " : "", expr(i + 1), ")"];
      case "SpreadElement":
        return ["...", expr(i + 1)];
      case "YieldExpression":
        return ["(yield ", expr(i + 1), ")"];
      case "SequenceExpression": {
        const k = n.expressions.indexOf(next);
        return ["(", join([expr(i + 1), ...n.expressions.slice(k + 1).map(rest)], ", "), ")"];
      }
      case "TemplateLiteral": {
        // Parts evaluated so far are strings already (ToString applied); djs keeps the
        // literal parts before the site too (`template.part@<quasi>`).
        const out = [cm(`template.part@${n.quasis[0].start - 1}`, "template.part"), "`"];
        let seen = false;
        n.quasis.forEach((q, k) => {
          if (!seen && hasSlot(`template.part@${q.start - 1}`)) used.add(`template.part@${q.start - 1}`);
          out.push(q.value.raw);
          if (k < n.expressions.length) {
            const e = n.expressions[k];
            const c = cm(`template.part@${e.start}`, "template.part");
            if (e === next) {
              out.push("${", c, expr(i + 1), "}");
              seen = true;
            } else if (seen) out.push("${", c, rest(e), "}");
            else out.push("${", c, tmp("template.part", e.start), "}");
            if (k + 1 < n.quasis.length) out.push(cm(`template.part@${n.quasis[k + 1].start - 1}`, "template.part"));
          }
        });
        out.push("`");
        return out;
      }
      case "TaggedTemplateExpression":
        refuse("tagged template around a site");
      case "ArrayExpression": {
        if (hasSlot(`array.literal@${n.start}`)) {
          const k = n.elements.indexOf(next);
          if (n.elements.slice(k + 1).some((e) => e === null)) refuse("a hole after the site in a kept array");
          const v = tmp("array.literal", n.start);
          const more = n.elements.slice(k + 1).map((e) => rest(e));
          return [ign("call.callee", "call.this"), "$djs_Q(", cm(`array.literal@${n.start}`, "call.arg"), v, ", ", join([expr(i + 1), ...more], ", "), ")"];
        }
        const els = [];
        let seen = false;
        for (const e of n.elements) {
          const c = e ? cm(`array.element@${e.start}`, "array.element") : "";
          if (e === next) {
            els.push([c, expr(i + 1)]);
            seen = true;
          } else if (seen) els.push(e ? [c, rest(e)] : "");
          else if (e === null) els.push("");
          else if (e.type === "SpreadElement") refuse("spread in an array before the site");
          else els.push([c, tmp("array.element", e.start)]);
        }
        return ["[", join(els, ", "), "]"];
      }
      case "MemberExpression":
        if (n.object.type === "Super") {
          if (!n.computed || next !== n.property) refuse("super around a site");
          const st = superThis ?? tmp("super.this", n.start);
          const sb = tmp("super.base", n.start);
          return ["$djs_G(", sb, ", ", expr(i + 1), ", ", st, ")"];
        }
        if (next === n.object) return [expr(i + 1), n.computed ? ["[", rest(n.property), "]"] : [n.optional ? "?." : ".", P(n.property)]];
        return [cm(`member.object@${n.start}`, "member.object"), tmp("member.object", n.start), "[", expr(i + 1), "]"];
      case "AssignmentExpression": {
        if (next !== n.right) refuse("site in an assignment target");
        if (n.left.type === "Identifier") {
          if (n.operator === "=" || n.operator.length === 3 && /[&|?]/.test(n.operator[0])) return ["(", P(n.left), " = ", expr(i + 1), ")"];
          return ["(", P(n.left), " = ", cm(`assign.old@${n.start}`, "binary.left"), tmp("assign.old", n.start), ` ${n.operator.slice(0, -1)} `, expr(i + 1), ")"];
        }
        if (n.left.type !== "MemberExpression" || n.left.object.type === "Super") refuse("assignment to a pattern or super");
        if (/^(&&|\|\||\?\?)=$/.test(n.operator)) refuse("logical assignment to a member around a site");
        const o = tmp("assign.object", n.start);
        let target;
        if (n.left.computed) {
          const k = tmpOpt("assign.key", n.start);
          target = [o, "[", k ?? rest(n.left.property), "]"];
        } else target = [o, ".", P(n.left.property)];
        // A member target evaluated before the site: assign through Reflect-free plain code.
        const marks = [cm(`assign.object@${n.start}`, "assign.object"), cm(`assign.key@${n.start}`, "assign.key")];
        if (n.operator === "=") return ["(", marks, target, " = ", expr(i + 1), ")"];
        return ["(", marks, target, " = ", cm(`assign.old@${n.start}`, "binary.left"), tmp("assign.old", n.start), ` ${n.operator.slice(0, -1)} `, expr(i + 1), ")"];
      }
      case "ObjectExpression": {
        // The object made so far; the rest of the literal is defined on it in order.
        const o = tmp("object.literal", n.start);
        const out = [];
        let seen = false;
        for (const p of n.properties) {
          if (!seen && p !== next) continue;
          const oo = [cm(`object.literal@${n.start}`, "call.arg"), o];
          const head = (h) => [ign("call.callee", "call.this"), h];
          if (p.type === "SpreadElement") {
            if (p === next) out.push([head("$djs_A("), oo, ", ", expr(i + 2), ")"]);
            else out.push([head("$djs_A("), oo, ", ", rest(p.argument), ")"]);
            seen = true;
            continue;
          }
          if (p.kind !== "init" || p.method) refuse("accessor or method after the site");
          if (!p.computed && (p.key.name ?? p.key.value) === "__proto__" && !p.shorthand) refuse("__proto__ in a literal around a site");
          let key;
          if (p.computed) key = p === next ? tmpOpt("object.key", p.key.start - 1) ?? rest(p.key) : rest(p.key);
          else key = JSON.stringify(String(p.key.name ?? p.key.value));
          if (p === next) {
            seen = true;
            if (path[i + 2] !== p.value) refuse("site in a computed key");
            out.push([head("$djs_D("), oo, ", ", ign("call.arg"), key, ", ", expr(i + 2), ")"]);
          } else out.push([head("$djs_D("), oo, ", ", ign("call.arg"), key, ", ", rest(p.value), ")"]);
        }
        return ["(", join(out, ", "), ", ", o, ")"];
      }
      case "Property":
        return expr(i + 1);
      default:
        refuse(`expression ${n.type} around a site`);
    }
  }

  // ---- statements -----------------------------------------------------------------------
  // A statement list from child `j` (which holds the site) on; `decls` the bindings the
  // block declares, `scopeNode` the node naming its scope.
  function list(stmts, j, i, decls, scopeNode) {
    const out = [declare(decls, scopeNode)];
    // The callback of the dump and the statement at the site.
    out.push(stmt(i));
    for (let k = j + 1; k < stmts.length; k++) {
      if (stmts[k].type === "FunctionDeclaration") continue; // hoisted: a binding of the prefix
      out.push(rest(stmts[k]));
    }
    return join(out.filter((x) => x !== ""), "\n");
  }
  const blockDecls = (stmts, j) => {
    const out = [];
    stmts.forEach((s, k) => {
      for (const d of lexDecls(s)) out.push({ ...d, live: k < j || s.type === "FunctionDeclaration" });
    });
    return out;
  };

  let tempBase = 0;
  const withGo = (parts) => {
    // parts are built: the temporaries they use are known
    const t = flushTemps();
    if (goDone || !ctx.go) return [t, parts];
    goDone = true;
    return [ctx.go, "\n", t, parts];
  };

  function stmt(i) {
    const n = path[i];
    const next = path[i + 1];
    switch (n.type) {
      case "BlockStatement": {
        const j = n.body.indexOf(next);
        return ["{\n", list(n.body, j, i + 1, blockDecls(n.body, j), n), "\n}"];
      }
      case "ExpressionStatement":
        return withGo([expr(i + 1), ";"]);
      case "ReturnStatement":
        return withGo(["return ", expr(i + 1), ";"]);
      case "ThrowStatement":
        return withGo(["throw ", expr(i + 1), ";"]);
      case "VariableDeclaration": {
        const parts = [];
        let seen = false;
        for (const d of n.declarations) {
          if (d === next) {
            if (path[i + 2] !== d.init) refuse("site in a pattern");
            parts.push([P(d.id), " = ", expr(i + 2)]);
            seen = true;
          } else if (seen) parts.push(rest(d));
          else
            patNames(d.id).forEach((m) => {
              const s = bindingSlot(m, n);
              parts.push(`${m} = ${s ? take(s) : "undefined"}`);
            });
        }
        if (n.kind === "var") refuse("site in a var declaration list");
        return withGo([n.kind, " ", join(parts, ", "), ";"]);
      }
      case "IfStatement":
        if (next === n.test) return withGo(["if (", expr(i + 1), ") ", rest(n.consequent), n.alternate ? [" else ", rest(n.alternate)] : ""]);
        return stmt(i + 1);
      case "LabeledStatement":
        return [P(n.label), ": ", stmt(i + 1)];
      case "ForOfStatement":
      case "ForInStatement":
        return forOf(n, i);
      case "WhileStatement": {
        if (next !== n.body) refuse("site in a while test");
        const f = `$djs_f${i}`;
        return [`{ let ${f} = true; `, markScaf("while", n.start), `while (${f} || (`, rest(n.test), `)) { if (${f}) { ${f} = false; `, stmt(i + 1), " } else ", rest(n.body), " } }"];
      }
      case "DoWhileStatement": {
        if (next !== n.body) refuse("site in a do-while test");
        const f = `$djs_f${i}`;
        return [`{ let ${f} = true; `, markScaf("do", n.start), `do { if (${f}) { ${f} = false; `, stmt(i + 1), " } else ", rest(n.body), " } while (", rest(n.test), "); }"];
      }
      case "ForStatement": {
        if (next !== n.body) refuse("site in a for head");
        const f = `$djs_f${i}`;
        let init = "";
        let decls = [];
        if (n.init?.type === "VariableDeclaration") {
          if (n.init.kind === "var") refuse("for (var ...) around a site");
          decls = n.init.declarations.flatMap((d) => patNames(d.id)).map((name) => ({ name, kind: n.init.kind, live: true }));
          init = `${n.init.kind} ${decls.map(({ name }) => `${name} = ${take(bindingSlot(name, n) ?? refuse(`no slot ${name}`))}`).join(", ")}`;
        } else if (n.init) init = ""; // ran already
        // the per-iteration scope: its bindings are declared by the head, the code the dump
        // puts there runs in the first iteration
        const own = declare(decls.map((d) => ({ ...d, live: false })), n);
        return [
          `{ let ${f} = true; `,
          markScaf("for", n.start),
          `for (${init}; ${f} || (`,
          n.test ? rest(n.test) : "true",
          "); ",
          n.update ? rest(n.update) : "",
          `) { if (${f}) { ${f} = false; `,
          own,
          "\n",
          stmt(i + 1),
          " } else ",
          rest(n.body),
          " } }",
        ];
      }
      case "TryStatement": {
        if (next === n.block) {
          return [
            markScaf("try", n.start),
            "try ",
            stmt(i + 1),
            n.handler ? [" catch", n.handler.param ? [" (", P(n.handler.param), ")"] : "", " ", rest(n.handler.body)] : "",
            n.finalizer ? [" finally ", rest(n.finalizer)] : "",
          ];
        }
        if (next === n.handler) {
          const names = n.handler.param ? patNames(n.handler.param) : [];
          const h = ["{\n", declare(names.map((name) => ({ name, kind: "let", live: true })), n.handler), "\n", stmt(i + 2), "\n}"];
          return n.finalizer ? [markScaf("try", n.start), "try ", h, " finally ", rest(n.finalizer)] : [markScaf("catch", n.start), h];
        }
        // In the finally block: the rest of it, then the completion that entered it.
        const kind = Number(inline("completion.kind", n.start));
        const v = inline("completion.value", n.start);
        const cv = `$djs_c${i}`;
        let done;
        if (kind === 0) done = "";
        else if (kind === 1) done = `throw ${cv};`;
        else if (kind === 2) done = `return ${cv};`;
        else refuse("finally entered by break or continue");
        const rec = markScaf("finally", n.start);
        scaf.at(-1).push(kind, cv);
        return [rec, `{ const ${cv} = `, v, "; ", stmt(i + 1), " ", done, " }"];
      }
      case "SwitchStatement": {
        const m = n.cases.findIndex((c) => c.start <= site.start && site.end <= c.end);
        const c = n.cases[m];
        if (c.test && c.test.start <= site.start && site.end <= c.test.end) refuse("site in a case test");
        const ci = path.indexOf(c);
        const before = n.cases.slice(0, m);
        const decls = before.flatMap((b) => b.consequent.flatMap(lexDecls)).map((d) => ({ ...d, live: true }));
        const laterDecls = n.cases.slice(m + 1).flatMap((b) => b.consequent.flatMap(lexDecls)).map((d) => ({ ...d, live: false }));
        const later = n.cases.slice(m + 1).flatMap((x) => x.consequent).map(rest);
        const j = c.consequent.indexOf(path[ci + 1]);
        return [markScaf("switch", n.start), "switch (0) { case 0: ", list(c.consequent, j, ci + 1, [...decls, ...blockDecls(c.consequent, j), ...laterDecls], n), "\n", join(later, "\n"), " }"];
      }
      default:
        refuse(`statement ${n.type} around a site`);
    }
  }

  // A for-of around the site: the rest of this iteration runs first, then the loop goes
  // on over the saved iterator record in the original body. The loop is a real for-of
  // over `$djs_L(iterator, next)`, an iterable whose first step is a placeholder, so
  // `break`, `continue`, `return` and exceptions close the iterator as the original's.
  function forOf(n, i) {
    const next = path[i + 1];
    if (n === site) return forAwaitHead(n, i);
    if (next !== n.body) refuse("site in a for-of head");
    const isAwait = n.await;
    const forIn = n.type === "ForInStatement";
    const c = isAwait ? "for-await" : forIn ? "for-in" : "for-of";
    let it = inline(`${c}.iterator`, n.start);
    let nx = forIn ? "null" : inline(`${c}.next`, n.start);
    if (isAwait) {
      // A for await over a sync iterable steps the iterator djs made for it; plain code gets
      // the same steps from a for await over the sync iterator itself.
      const m = /^\$dump\.fromSync\((.*), (.*)\)$/s.exec(it);
      if (!m) refuse("for await over an async iterator");
      it = m[1];
      nx = m[2];
    }
    const f = `$djs_f${i}`;
    const v = `$djs_v${i}`;
    const head = n.left;
    let bind;
    let decls = [];
    if (head.type === "VariableDeclaration") {
      if (head.kind === "var") refuse("for (var ... of) around a site");
      decls = patNames(head.declarations[0].id).map((name) => ({ name, kind: head.kind, live: true }));
      bind = [head.kind, " ", P(head.declarations[0].id), ` = ${v};`];
    } else bind = [P(head), ` = ${v};`];
    return [
      `{ let ${f} = true; `,
      markScaf(c, n.start),
      `for${isAwait ? " await" : ""} (const ${v} of $djs_L(`,
      it,
      ", ",
      nx,
      `)) { if (${f}) { ${f} = false; {\n`,
      declare(decls, n),
      "\n",
      stmt(i + 1),
      "\n} } else { ",
      bind,
      " ",
      rest(n.body),
      " } } }",
    ];
  }

  // The site is a for await's step (`for-await.next`): the loop goes on over the saved
  // iterator record, its first step being the promise the frame awaits.
  function forAwaitHead(n, i) {
    const it = inline("for-await.iterator", n.start);
    const nx = inline("for-await.next", n.start);
    if (/^\$dump\.fromSync/.test(it)) refuse("for await over a sync iterator at its step");
    const v = `$djs_v${i}`;
    const head = n.left;
    let bind;
    if (head.type === "VariableDeclaration") bind = [head.kind, " ", P(head.declarations[0].id), ` = ${v};`];
    else bind = [P(head), ` = ${v};`];
    return withGo([markScaf("site", n.start), `for await (const ${v} of $djs_N(`, it, ", ", nx, ", ", ctx.awaiting, ")) { ", bind, " ", rest(n.body), " }"]);
  }

  // ---- the function -----------------------------------------------------------------------
  let body;
  if (fn.body.type === "BlockStatement") {
    const j = fn.body.body.indexOf(path[1]);
    body = list(fn.body.body, j, 1, functionDecls(j), fn);
  } else {
    // an arrow with an expression body
    body = [declare(functionDecls(0), fn), "\n", withGo(["return ", expr(0), ";"])];
  }
  // What K captures at its start, while the dump loads (user code cannot have replaced the
  // builtins yet).
  const helpers = { mark: null, used: true };
  const kind = fn.async && fn.generator ? "async function* " : fn.async ? "async function " : fn.generator ? "function* " : "function ";
  const head = isArrow ? [fn.async ? "async " : "", "() => {\n"] : [kind, "() {\n"];
  // Only the helpers K uses are captured (filled in by flatten's caller via `helpersFor`).
  const parts = [head, { helpers: true }, body, "\n}"];
  return { parts, scaf, used, usesArguments, tempNames, cmap };
}

// ---- the dump -------------------------------------------------------------------------------
// Helpers the plain dump defines on `$dump` (a plain global object, deleted at the end of
// the load): same names as the loader primitives where the primitive has a plain encoding.
const HELPERS = {
  R: "Reflect.apply",
  G: "Reflect.get",
  C: "Reflect.construct",
  D: "(o, k, v) => Object.defineProperty(o, k, { value: v, writable: true, enumerable: true, configurable: true })",
  A: "Object.assign",
  // An iterable over a saved iterator record whose first step is a placeholder: the rest
  // of the current iteration runs in it, the next ones take the iterator's steps.
  L: `(it, next) => { let first = true; const r = { next() { if (first) { first = false; return { value: undefined, done: false }; } return Reflect.apply(next ?? it.next, it, []); }, return(v) { const m = it.return; return m == null ? { done: true, value: v } : Reflect.apply(m, it, [v]); }, [Symbol.iterator]() { return r; } }; return r; }`,
  N: `(it, next, p) => { let first = true; const r = { next() { if (first) { first = false; return p; } return Reflect.apply(next, it, []); }, return(v) { const m = it.return; return m == null ? { done: true, value: v } : Reflect.apply(m, it, [v]); }, [Symbol.asyncIterator]() { return r; } }; return r; }`,
  regexpIterator: "(re, s, f) => { const it = $dump.MA.call(re, s); if (f & 4) while (!it.next().done); return it; }",
  MA: "RegExp.prototype[Symbol.matchAll]",
  // a for-in loop's key iterator over o with the keys it has yet to visit (deleted ones are
  // skipped, as the loop does)
  forIn: "(o, keys) => { let i = 0; const r = { next() { while (i < keys.length) { const k = keys[i++]; if (o == null || k in o) return { value: k, done: false }; } return { value: undefined, done: true }; }, [Symbol.iterator]() { return r; } }; return r; }",
  Q: "(a, ...v) => { let n = a.length; for (const x of v) Object.defineProperty(a, n++, { value: x, writable: true, enumerable: true, configurable: true }); return a; }",
  advance: "(it, n) => { const next = it.next; for (let i = 0; i < n; i++) Reflect.apply(next, it, []); return it; }",
  fn: "(o) => { const d = Object.getOwnPropertyDescriptor(o, Reflect.ownKeys(o)[0]); return 'value' in d ? d.value : d.get ?? d.set; }",
  spent: "(k) => { let f; new Promise((a, b) => { f = k ? b : a; a(); }); return f; }",
  rejected: "(e) => { const p = Promise.reject(e); p.then(undefined, () => {}); return p; }",
  bytes: "(b) => Uint8Array.fromBase64(b).buffer",
  keep: "(o, keys) => { for (const k of Reflect.ownKeys(o)) if (!['length', 'name', 'prototype', 'constructor'].includes(k) && !keys.includes(k) && !delete o[k]) throw new TypeError('keep'); }",
};
// Intrinsics without a global name, for $dump.builtin paths.
const INTRINSICS = {
  "%ArrayIteratorPrototype%": "Object.getPrototypeOf([][Symbol.iterator]())",
  "%MapIteratorPrototype%": "Object.getPrototypeOf(new Map()[Symbol.iterator]())",
  "%SetIteratorPrototype%": "Object.getPrototypeOf(new Set()[Symbol.iterator]())",
  "%StringIteratorPrototype%": 'Object.getPrototypeOf(""[Symbol.iterator]())',
  "%RegExpStringIteratorPrototype%": 'Object.getPrototypeOf(/(?:)/[Symbol.matchAll](""))',
  "%GeneratorPrototype%": "Object.getPrototypeOf(function* () {}).prototype",
  "%AsyncGeneratorPrototype%": "Object.getPrototypeOf(async function* () {}).prototype",
  "%IteratorPrototype%": "Object.getPrototypeOf(Object.getPrototypeOf([][Symbol.iterator]()))",
  "%AsyncIteratorPrototype%": "Object.getPrototypeOf(Object.getPrototypeOf(async function* () {}).prototype)",
  "%IteratorHelperPrototype%": "Object.getPrototypeOf([].values().map((x) => x))",
  "%WrapForValidIteratorPrototype%": "Object.getPrototypeOf(Iterator.from({ next() {} }))",
  "%TypedArray%": "Object.getPrototypeOf(Uint8Array)",
};
function builtinExpr(path) {
  let getter = null;
  let p = path;
  const m = /^(get|set) (.*)$/.exec(p);
  if (m) [, getter, p] = m;
  const parts = p.split(".");
  let base;
  if (parts[0].startsWith("%")) base = INTRINSICS[parts[0]] ?? refuse(`intrinsic ${parts[0]}`);
  else base = parts[0];
  const last = parts.pop();
  const obj = [base, ...parts.slice(1)].join(".");
  const key = last.startsWith("@@") ? `Symbol.${last.slice(2)}` : JSON.stringify(last);
  if (getter) return `Object.getOwnPropertyDescriptor(${obj}, ${key}).${getter}`;
  if (parts.length === 0) return base;
  return `${obj}[${key}]`;
}

const memberChain = (n) => {
  // `$dump.a.b.c` -> ["a", "b", "c"]
  const out = [];
  while (n.type === "MemberExpression" && !n.computed) {
    out.unshift(n.property.name);
    n = n.object;
  }
  return n.type === "Identifier" && n.name === "$dump" ? out : null;
};

export function toPlain(text, opts = {}) {
  const ast = parseScript(text);
  const src = (n) => text.slice(n.start, n.end);
  const stats = { activations: 0, merged: 0, callbacks: 0 };
  const builtins = new Map();
  const ktexts = new Map();
  const omittedDefs = new Set();
  if (!/^"djs-dump 2( typescript)?";/.test(text)) refuse("not a format 2 dump");
  if (text.startsWith('"djs-dump 2 typescript";')) refuse("TypeScript text");
  for (const p of ["private", "closeThrow", "using", "iteratorHelper"])
    if (text.includes(`$dump.${p}(`)) refuse(`$dump.${p}`);

  // -- factories --------------------------------------------------------------------------
  const factories = new Map(); // top-level name -> factory
  const allFactories = [];
  const isFactoryCall = (n) =>
    n?.type === "CallExpression" && n.arguments.length === 0 && n.callee.type === "ArrowFunctionExpression" && n.callee.body.type === "BlockStatement";
  function parseFactory(name, call, parent, top) {
    const F = { name, call, parent, top: top ?? null, subs: [], vars: [], ret: null, props: new Map(), cells: new Map(), spreads: [], other: [], ks: [], omit: new Set() };
    F.top = top ?? F;
    allFactories.push(F);
    let afterReturn = false;
    for (const st of call.callee.body.body) {
      if (st.type === "ReturnStatement") {
        F.ret = st.argument;
        afterReturn = true;
      } else if (st.type === "VariableDeclaration" && st.declarations.length === 1 && isFactoryCall(st.declarations[0].init)) {
        F.subs.push(parseFactory(st.declarations[0].id.name, st.declarations[0].init, F, F.top));
      } else if (st.type === "VariableDeclaration" && st.kind === "let") {
        for (const d of st.declarations) F.vars.push({ name: d.id.name, init: afterReturn ? null : d.init ? d.init : null, tdz: afterReturn });
      } else F.other.push(st);
    }
    if (!F.ret || F.ret.type !== "ObjectExpression") refuse("factory without an object");
    for (const p of F.ret.properties) {
      if (p.type === "SpreadElement") F.spreads.push(p.argument.name);
      else {
        const k = p.key.type === "Identifier" ? p.key.name : p.key.value;
        if (p.value.type === "CallExpression" && src(p.value.callee) === "$dump.cell") F.cells.set(k, p.value.arguments[0].body.name);
        else F.props.set(k, p);
      }
    }
    return F;
  }
  // Finds key `k` among F's own props/cells or its spread subs: [factory, kind].
  function findKey(F, k) {
    if (F.props.has(k)) return [F, "prop"];
    if (F.cells.has(k)) return [F, "cell"];
    for (const s of F.subs) {
      if (!F.spreads.includes(s.name)) continue;
      const r = findKey(s, k);
      if (r) return r;
    }
    return null;
  }

  // -- statements -------------------------------------------------------------------------
  const items = [];
  const globals = new Set(); // names a top-level declaration of the dump binds
  const defs = new Map(); // $dump name -> statement node
  const acts = [];
  const fnDecls = [];
  let handle = null;
  for (const st of ast.body) {
    if (st.type === "ExpressionStatement" && st.directive) continue;
    if (st.type === "FunctionDeclaration") {
      globals.add(st.id.name);
      fnDecls.push(st);
      continue;
    }
    if (st.type === "VariableDeclaration" || st.type === "ClassDeclaration") {
      (st.type === "ClassDeclaration" ? [st.id.name] : st.declarations.flatMap((d) => patNames(d.id))).forEach((n) => globals.add(n));
      items.push({ kind: "decl", node: st });
      continue;
    }
    if (st.type !== "ExpressionStatement") refuse(`top-level ${st.type}`);
    const e = st.expression;
    if (e.type === "CallExpression" && src(e.callee) === "$dump.handle") {
      if (e.arguments[0].value !== "run") refuse(`handle ${e.arguments[0].value}`);
      handle = e.arguments[1];
      continue;
    }
    if (e.type === "CallExpression" && /^\$dump\.(random|issued)$/.test(src(e.callee))) continue;
    if (e.type === "AssignmentExpression" && e.operator === "=" && memberChain(e.left)?.length === 1) {
      const name = memberChain(e.left)[0];
      defs.set(name, st);
      if (isFactoryCall(e.right)) {
        const F = parseFactory(name, e.right, null, null);
        factories.set(name, F);
        items.push({ kind: "factory", node: st, F, name });
        continue;
      }
      if (e.right.type === "CallExpression" && src(e.right.callee) === "$dump.resume") {
        const A = { name, node: st, call: e.right };
        acts.push(A);
        items.push({ kind: "act", node: st, A, name });
        continue;
      }
      items.push({ kind: "def", node: st, name });
      continue;
    }
    items.push({ kind: "stmt", node: st });
  }
  if (!handle) refuse("no run handle");

  // -- activations ----------------------------------------------------------------------------
  const objProps = (o) => {
    const m = new Map();
    for (const p of o.properties) m.set(p.key.type === "Identifier" ? p.key.name : p.key.value, p.value);
    return m;
  };
  // A $dump.sN.key reference -> {F, kind} (the factory whose own object holds the key).
  function factoryRef(n) {
    const ch = memberChain(n);
    if (!ch || ch.length !== 2 || !factories.has(ch[0])) return null;
    const r = findKey(factories.get(ch[0]), ch[1]);
    return r ? { F: r[0], kind: r[1], key: ch[1], top: ch[0] } : null;
  }
  // The function of an activation: {text, home (factory or null), ref (D text or null)}.
  function resolveFn(n) {
    if (isFn(n)) return { text: src(n), home: null, ref: null };
    const fr = factoryRef(n);
    if (fr) {
      if (fr.kind !== "prop") refuse("activation of a cell");
      const p = fr.F.props.get(fr.key);
      return { text: p.method || p.kind !== "init" ? src(p) : src(p.value), home: fr.F, ref: src(n), key: fr.key };
    }
    const ch = memberChain(n);
    if (ch && ch.length === 1 && defs.has(ch[0])) {
      const r = defs.get(ch[0]).expression.right;
      if (isFn(r)) return { text: src(r), home: null, ref: src(n) };
      if (r.type === "CallExpression" && src(r.callee) === "$dump.fn") return { text: src(r.arguments[0].properties[0]), home: null, ref: src(n) };
      if (r.type === "ClassExpression") refuse("class method activation");
    }
    if (n.type === "CallExpression" && src(n.callee) === "$dump.fn") return { text: src(n.arguments[0].properties[0]), home: null, ref: null, inline: n };
    const cm = classMethod(n);
    if (cm) return cm;
    if (n.type === "Identifier" && globals.has(n.name)) {
      const d = fnDecls.find((f) => f.id.name === n.name);
      if (d) return { text: src(d), home: null, ref: n.name };
      for (const it of items)
        if (it.kind === "decl" && it.node.type === "VariableDeclaration")
          for (const dd of it.node.declarations) if (dd.id.name === n.name && dd.init && (isFn(dd.init) || dd.init.type === "CallExpression")) {
            if (isFn(dd.init)) return { text: src(dd.init), home: null, ref: n.name };
            if (src(dd.init.callee) === "$dump.fn") return { text: src(dd.init.arguments[0].properties[0]), home: null, ref: n.name };
          }
    }
    refuse(`activation of ${src(n).slice(0, 40)}`);
  }

  // A class method: `C.prototype.m` or `C.m`, C a class the dump writes (a top-level class,
  // or a factory's). K is written where the class is; names only the class's own scope
  // binds (its name, an enclosing wrapper's parameters) are refused.
  function classMethod(n) {
    if (n.type !== "MemberExpression" || n.computed) return null;
    const m = n.property.name;
    let obj = n.object;
    let isStatic = true;
    if (obj.type === "MemberExpression" && !obj.computed && obj.property.name === "prototype") {
      isStatic = false;
      obj = obj.object;
    }
    let cls = null;
    let home = null;
    let wrapperParams = [];
    const fromInit = (init) => {
      if (!init) return null;
      if (init.type === "ClassExpression") return init;
      // ((Base) => class C extends Base {...})(Base)
      if (init.type === "CallExpression" && init.callee.type === "ArrowFunctionExpression" && init.callee.body.type === "ClassExpression") {
        wrapperParams = init.callee.params.flatMap((p) => patNames(p));
        return init.callee.body;
      }
      return null;
    };
    if (obj.type === "Identifier") {
      for (const st of ast.body) {
        if (st.type === "ClassDeclaration" && st.id.name === obj.name) cls = st;
        if (st.type === "VariableDeclaration") for (const d of st.declarations) if (d.id.name === obj.name) cls = fromInit(d.init) ?? cls;
      }
    } else {
      const fr = factoryRef(obj);
      if (fr && fr.kind === "prop") {
        const p = fr.F.props.get(fr.key);
        let v = p.value;
        if (v.type === "Identifier") {
          const V = fr.F.vars.find((x) => x.name === v.name);
          cls = fromInit(V?.init);
        } else cls = fromInit(v);
        home = fr.F;
      }
    }
    if (!cls) return null;
    const md = cls.body.body.find((x) => x.type === "MethodDefinition" && !x.computed && (x.key.name ?? x.key.value) === m && !!x.static === isStatic && x.kind === "method");
    if (!md) return null;
    const free = freeVars(md.value);
    const own = [...wrapperParams, cls.id?.name].filter(Boolean);
    if (own.some((x) => free.has(x))) refuse("a class method that names its class's own scope");
    return { text: src(md), home, ref: src(n), key: null, placeNode: cls, classRef: src(obj) };
  }

  // ---- origins that are only origins ------------------------------------------------------
  // A function nothing but its activation holds (a codemode body, a callback the frame
  // runs) is not written as a function: K's origin is its text, pruned of the top-level
  // statements before the site (they never run again; declarations keep a skeleton for
  // their names and kinds). The dump then carries no code that already ran, which is why
  // R's dumps were smaller than today's.
  function originOnly(A) {
    if (A.f.inline) return false;
    if (!A.f.ref) return true;
    const ref = A.f.ref;
    if (/^\$dump\.[a-z]+\d+$/.test(ref)) {
      // a top-level definition only the activation names
      const name = ref.slice(6);
      const uses = ast.body.filter((st) => st !== A.node && st !== defs.get(name) && src(st).includes(ref) && !(st.expression?.type === "CallExpression" && src(st.expression.callee) === "Object.defineProperty" && src(st.expression.arguments[0]) === ref));
      if (uses.length) return false;
      A.f.omitDef = name;
      omittedDefs.add(name);
      return true;
    }
    if (!/^\$dump\.s\d+\.[\w$]+$/.test(ref)) return false;
    // no other reference than the activation and name patches
    let n = 0;
    for (const st of ast.body) {
      const t = src(st);
      if (!t.includes(ref)) continue;
      if (st === A.node) continue;
      if (st.expression?.type === "CallExpression" && src(st.expression.callee) === "Object.defineProperty" && src(st.expression.arguments[0]) === ref) continue;
      n += t.split(ref).length - 1;
      // the factory holding it defines it, which is not a use
      const [, top, key] = /^\$dump\.(s\d+)\.([\w$]+)$/.exec(ref);
      if (st.expression?.type === "AssignmentExpression" && memberChain(st.expression.left)?.[0] === top) n -= t.split(ref).length - 1;
    }
    if (n > 0) return false;
    const [, top, key] = /^\$dump\.(s\d+)\.([\w$]+)$/.exec(ref);
    // a function some other code of its factory refers to by key is not only an origin
    A.f.omitKey = key;
    A.f.home.omit.add(key);
    return true;
  }
  function prune(A) {
    const fp = parseFunctionText(A.f.text);
    const fn = fp.fn;
    const [kind, nth] = A.at.split("#");
    const site = sites(fn)[kind]?.[Number(nth) - 1];
    if (!site || fn.body.type !== "BlockStatement" || (kind !== "await" && kind !== "yield")) return;
    const body = fn.body.body;
    const j = body.findIndex((st) => st.start <= site.start && site.end <= st.end);
    if (j <= 0) return;
    const t = A.f.text;
    const skel = [];
    const vars = [];
    for (const st of body.slice(0, j)) {
      if (st.type === "VariableDeclaration" && st.kind !== "var") skel.push(`${st.kind} ${st.declarations.map((d) => `${t.slice(d.id.start, d.id.end)} = 0`).join(", ")};`);
      else if (st.type === "FunctionDeclaration") skel.push(`${st.async ? "async " : ""}function${st.generator ? "*" : ""} ${st.id.name}() {}`);
      else if (st.type === "ClassDeclaration") skel.push(`class ${st.id.name} {}`);
      varNames(st).forEach((v) => vars.push(v));
    }
    if (vars.length) skel.unshift(`var ${[...new Set(vars)].join(", ")};`);
    const head = t.slice(0, body[0].start) + skel.join("\n") + "\n";
    const delta = body[j].start - head.length;
    if (delta <= 0) return;
    A.f.text = head + t.slice(body[j].start);
    const map = (S) => (S >= body[j].start ? S - delta : S);
    const renamed = new Map();
    for (const [k, v] of A.slots) {
      const m = /^(.*)@(\d+)$/.exec(k);
      renamed.set(m ? `${m[1]}@${map(Number(m[2]))}` : k, v);
    }
    A.slots = renamed;
    const fs2 = sites(parseFunctionText(A.f.text).fn)[kind];
    A.at = `${kind}#${fs2.findIndex((x) => x.start === site.start - delta) + 1}`;
    A.f.pruned = true;
  }

  // ---- merged factories: variables a frame shares with closures ---------------------------
  // A factory one of whose variables an activation takes as a cell is "merged": its
  // variables become declarations of continuations, its closures are made inside them.
  const merged = new Set();
  for (const A of acts) {
    const [fnNode, optsNode] = A.call.arguments;
    A.opts = objProps(optsNode);
    A.slots = A.opts.has("slots") ? objProps(A.opts.get("slots")) : new Map();
    A.f = resolveFn(fnNode);
    A.at = A.opts.get("at").value;
    if (opts.prune !== false && originOnly(A)) prune(A);
    A.cells = new Map(); // slot name -> {F, V}
    for (const [k, v] of A.slots) {
      const fr = factoryRef(v);
      if (fr?.kind === "cell") {
        const varName = fr.F.cells.get(fr.key);
        const V = fr.F.vars.find((x) => x.name === varName) ?? refuse(`cell ${fr.key} without its variable`);
        A.cells.set(k, { F: fr.F, V });
        V.owner = { A, sname: k };
        merged.add(fr.F);
      }
    }
    stats.activations++;
  }
  // The ancestors of a merged factory up to the first unmerged one are merged as well when
  // they share variables with it (scope chain): a factory nested in a merged one whose own
  // variables no frame takes keeps them as a dead scope inside the continuation.
  for (const F of [...merged]) for (let p = F.parent; p; p = p.parent) if (p.vars.some((v) => v.owner)) merged.add(p);
  stats.merged = merged.size;

  // Blocks of each activation (a dry run of the generator records them).
  const fnInfo = new Map();
  for (const A of acts) {
    const parsed = parseFunctionText(A.f.text);
    A.fn = parsed.fn;
    A.fkind = parsed.kind;
    const [k, nth] = A.at.split("#");
    if (k !== "await" && k !== "yield" && k !== "for-await.next") refuse(`site ${k}`);
    A.site = sites(A.fn)[k]?.[Number(nth) - 1] ?? refuse(`no site ${A.at}`);
    A.siteKind = k;
    A.blocks = [];
    const dry = gen(ctxFor(A, true));
    A.dryUsed = dry.used;
  }
  function ctxFor(A, dry) {
    return {
      ftext: A.f.text,
      fn: A.fn,
      site: A.site,
      awaiting: A.opts.has("awaiting") ? rw(A.opts.get("awaiting")) : "undefined",
      slot: (name) => (A.slots.has(name) ? slotValue(A, name) : undefined),
      afterGo: (name) => A.afterGo?.has(name) ?? false,
      cellOf: (sname) => {
        const c = A.cells.get(sname);
        if (!c) return null;
        return { init: c.V.tdz || c.V.init === null ? null : rw(c.V.init) };
      },
      blockContent: (index, info) => {
        if (dry) {
          A.blocks.push({ A, index, info, kind: "block" });
          return [];
        }
        return emitPlace(A.blocks[index]);
      },
      go: dry ? "" : A.go ?? "",
    };
  }
  // A slot's value as K's text: values that name globals are evaluated at the call (K's
  // own declarations would shadow them inside K).
  function slotValue(A, name) {
    const node = A.slots.get(name);
    const free = [...freeVars(node)].filter((n) => n !== "$dump" && n !== "undefined");
    if (!free.length || A.afterGo?.has(name)) return rw(node);
    A.pre ??= new Map();
    if (!A.pre.has(name)) A.pre.set(name, `$dump.${A.name}_${A.pre.size}`);
    return A.pre.get(name);
  }
  // The block of A declaring slot `sname`.
  for (const A of acts)
    for (const b of A.blocks) for (const d of b.info) if (d.sname && A.cells.has(d.sname)) A.cells.get(d.sname).V.block = b;
  for (const A of acts) for (const [k, c] of A.cells) if (!c.V.block) refuse(`no block declares ${k}`);

  // ---- placement --------------------------------------------------------------------------
  // A place is a block of a continuation, a dead scope group, an unmerged factory or the top.
  // parentOf(place) gives the enclosing place.
  const TOP = { kind: "top" };
  function parentOf(pl) {
    if (pl.kind === "block") return pl.index > 0 ? pl.A.blocks[pl.index - 1] : placeOfAct(pl.A);
    if (pl.kind === "group") return placeOfGroup(pl.F);
    if (pl.kind === "factory") return pl.F.parent ? factoryPlace(pl.F.parent) : TOP;
    return null;
  }
  const ancestors = (pl) => {
    const out = [];
    for (let p = pl; p; p = parentOf(p)) out.push(p);
    return out;
  };
  function innermost(pls, what) {
    const uniq = [...new Set(pls)];
    if (!uniq.length) return null;
    for (const c of uniq) {
      const anc = ancestors(c);
      if (uniq.every((o) => anc.includes(o))) return c;
    }
    refuse(`${what}: scopes of different frames`);
  }
  const fplaces = new Map();
  function factoryPlace(F) {
    // an unmerged factory is a place of its own; a merged one is not (its parts move)
    if (!merged.has(F)) {
      if (!fplaces.has(F)) fplaces.set(F, { kind: "factory", F });
      return fplaces.get(F);
    }
    return placeOfGroup(F);
  }
  // The variables a merged factory's code can capture: its own and its merged ancestors'.
  function scopeVars(F) {
    const out = new Map();
    for (let p = F; p && merged.has(p); p = p.parent) for (const v of p.vars) if (!out.has(v.name)) out.set(v.name, { v, F: p });
    return out;
  }
  // The place of code captured from merged factory F (a closure, a sub-factory, a K).
  const memo = new Map();
  const inProgress = new Set();
  function placeOfCode(F, node, extraFree) {
    const key = node;
    if (memo.has(key)) return memo.get(key);
    if (inProgress.has(key)) refuse("cyclic placement");
    inProgress.add(key);
    const free = extraFree ?? freeVars(node);
    const vars = scopeVars(F);
    const pls = [];
    for (const name of free) {
      const e = vars.get(name);
      if (!e) continue;
      pls.push(e.v.owner ? e.v.block : groupPlace(e.F));
    }
    let pl = innermost(pls, "a closure");
    if (!pl) {
      // captures nothing of the merged scopes: the nearest unmerged factory or the top
      let p = F;
      while (p && merged.has(p)) p = p.parent;
      pl = p ? factoryPlace(p) : TOP;
    }
    inProgress.delete(key);
    memo.set(key, pl);
    return pl;
  }
  // Dead variables of a merged factory (no frame takes them) form a group: a block in the
  // continuation that declares them, holding the closures that capture them.
  const groups = new Map();
  function groupPlace(F) {
    if (!groups.has(F)) groups.set(F, { kind: "group", F, members: [] });
    return groups.get(F);
  }
  const groupPlaceMemo = new Map();
  function placeOfGroup(F) {
    if (groupPlaceMemo.has(F)) return groupPlaceMemo.get(F);
    if (inProgress.has(F)) refuse("cyclic group placement");
    inProgress.add(F);
    // the innermost place among the live variables the group's code captures, and the
    // places of its own variables' initializers' owners
    const vars = scopeVars(F);
    const pls = [];
    const dead = new Set(F.vars.filter((v) => !v.owner).map((v) => v.name));
    for (const item of factoryCode(F)) {
      const free = freeVars(item.node);
      if (![...free].some((n) => dead.has(n))) continue;
      for (const name of free) {
        const e = vars.get(name);
        if (!e || (e.F === F && dead.has(name))) continue;
        pls.push(e.v.owner ? e.v.block : groupPlace(e.F) === groupPlace(F) ? null : placeOfGroup(e.F));
      }
    }
    let pl = innermost(pls.filter(Boolean), "a dead scope");
    if (!pl) {
      let p = F.parent;
      while (p && merged.has(p) && !p.vars.some((v) => !v.owner)) p = p.parent;
      pl = !p ? TOP : merged.has(p) ? groupPlace(p) : factoryPlace(p);
    }
    inProgress.delete(F);
    groupPlaceMemo.set(F, pl);
    return pl;
  }
  // The code of a factory that is placed: closures (props), unmerged subs, continuations.
  function factoryCode(F) {
    const out = [];
    for (const [key, p] of F.props) if (!F.omit.has(key)) out.push({ kind: "prop", key, node: p, F });
    for (const sub of F.subs) if (!merged.has(sub)) out.push({ kind: "sub", node: sub.call, sub, F });
    return out;
  }
  // Where each activation's K is written: with its function.
  const actPlace = new Map();
  function placeOfAct(A) {
    if (actPlace.has(A)) return actPlace.get(A);
    let pl;
    if (!A.f.home) pl = TOP;
    else if (!merged.has(A.f.home)) pl = factoryPlace(A.f.home);
    else {
      const p = A.f.placeNode ?? A.f.home.props.get(A.f.key);
      pl = codePlace({ kind: "prop", key: A.f.key, node: p, F: A.f.home });
    }
    actPlace.set(A, pl);
    return pl;
  }
  function codePlace(item) {
    const F = item.F;
    const dead = new Set(F.vars.filter((v) => !v.owner).map((v) => v.name));
    const free = freeVars(item.node);
    if ([...free].some((n) => dead.has(n))) return groupPlace(F);
    return placeOfCode(F, item.node, free);
  }

  // Everything placed, by place.
  const placed = new Map(); // place -> [{kind, ...}]
  const put = (pl, x) => {
    if (!placed.has(pl)) placed.set(pl, []);
    placed.get(pl).push(x);
  };
  for (const F of merged) {
    for (const item of factoryCode(F)) put(codePlace(item), item);
    if (F.vars.some((v) => !v.owner)) put(placeOfGroup(F), { kind: "group", G: groupPlace(F), F });
    if (F.other.length) refuse("factory with other statements");
  }
  for (const A of acts) {
    const pl = placeOfAct(A);
    if (pl.kind !== "top" && pl.kind !== "factory") put(pl, { kind: "k", A });
    if (pl.kind === "factory") pl.F.ks.push(A);
    A.place = pl;
  }

  const plan = schedule();
  return { text: emit(plan), stats };

  // ---- scheduling ---------------------------------------------------------------------------
  // Items run in the dump's order. A continuation is called where its activation was, but
  // code the dump wrote earlier may need what its prefix makes (the closures of a scope
  // its frame shares): then the continuation is called there, and what runs between there
  // and the activation's own place goes into a callback its prefix calls right before the
  // site's statement.
  function refsOf(node, skip) {
    const out = new Set();
    (function visit(n) {
      if (skip && skip.has(n)) return;
      const ch = n.type === "MemberExpression" ? memberChain(n) : null;
      if (ch && ch.length) {
        const name = ch[0];
        if (ch.length >= 2 && factories.has(name) && merged.has(factories.get(name))) out.add(`${name}.${ch[1]}`);
        else out.add(name);
        return;
      }
      for (const c of children(n)) visit(c);
    })(node);
    for (const g of freeVars(node)) if (globals.has(g) && !fnDecls.some((f) => f.id.name === g)) out.add("g:" + g);
    return out;
  }
  // The activation whose prefix makes key `key` of merged factory `name`.
  function creatorOf(name, key) {
    const F = factories.get(name);
    let pl = null;
    const r = findKey(F, key);
    if (r && r[1] === "prop" && r[0].omit.has(key)) return null;
    if (r && r[1] === "prop") {
      const item = factoryCode(r[0]).find((x) => x.kind === "prop" && x.key === key);
      pl = merged.has(r[0]) ? codePlace(item) : factoryPlace(r[0]);
    } else {
      // a continuation's key, or a key of an unmerged sub
      const A = acts.find((a) => `$K${a.name}` === key);
      if (A) pl = A.place;
      else
        for (const sub of allFactories)
          if (sub.top === F && !merged.has(sub) && sub.props.has(key)) pl = factoryPlace(sub);
    }
    for (let p = pl; p; p = parentOf(p)) {
      if (p.kind === "block") return p.A;
      if (p.kind === "factory" && !merged.has(p.F) && p.F.parent === null) return null;
    }
    return null;
  }
  function schedule() {
    const providers = new Map(); // ref -> item
    for (const it of items) {
      if (it.name) providers.set(it.name, it);
      if (it.kind === "decl") (it.node.type === "ClassDeclaration" ? [it.node.id.name] : it.node.declarations.flatMap((d) => patNames(d.id))).forEach((n) => providers.set("g:" + n, it));
    }
    const actItem = new Map(items.filter((x) => x.kind === "act").map((x) => [x.A, x]));
    // An async generator's queued requests are made again on its continuation: their
    // promises come from the activation's call, not from their placeholder definitions.
    for (const [A, it] of actItem) {
      A.queue = A.opts.has("queue") ? A.opts.get("queue").elements.map(objProps) : [];
      for (const q of A.queue) {
        const ch = memberChain(q.get("promise"));
        if (!ch || ch.length !== 1) refuse("a queued request's promise");
        const def = providers.get(ch[0]);
        if (def && def.kind === "def") {
          if (src(def.node.expression.right) !== "new Promise(() => {})") refuse("a queued request's promise is not a placeholder");
          def.skip = true;
        }
        providers.set(ch[0], it);
      }
    }
    const depItems = (refs, self) => {
      const out = [];
      for (const r of refs) {
        if (r.includes(".")) {
          const [n, k] = r.split(".");
          const A = creatorOf(n, k);
          if (A && actItem.get(A) !== self) out.push(actItem.get(A));
          continue;
        }
        const p = providers.get(r);
        if (p && p !== self) out.push(p);
      }
      return out;
    };
    // The deps of each item.
    for (const it of items) {
      if (it.kind === "act") {
        const A = it.A;
        const refs = new Set();
        for (const [k, v] of A.slots) if (!A.cells.has(k)) refsOf(v).forEach((r) => refs.add(r));
        if (A.opts.has("this")) refsOf(A.opts.get("this")).forEach((r) => refs.add(r));
        for (const c of A.cells.values()) if (c.V.init && !c.V.tdz) refsOf(c.V.init).forEach((r) => refs.add(r));
        // what its blocks hold (not nested continuations: they need nothing until called)
        const visitPlace = (pl) => {
          for (const x of placed.get(pl) ?? []) {
            if (x.kind === "prop") refsOf(x.node).forEach((r) => refs.add(r));
            if (x.kind === "sub") refsOf(x.node).forEach((r) => refs.add(r));
            if (x.kind === "group") {
              for (const v of x.F.vars) if (!v.owner && v.init && !v.tdz) refsOf(v.init).forEach((r) => refs.add(r));
              visitPlace(x.G);
            }
          }
        };
        A.blocks.forEach(visitPlace);
        it.deps = depItems(refs, it);
        // its K's definition
        if (A.place.kind === "factory") {
          let U = A.place.F;
          if (U.top !== U && merged.has(U.top)) {
            const c = creatorOf(U.top.name, [...U.props.keys()][0]);
            if (c) it.deps.push(actItem.get(c));
          } else it.deps.push(providers.get(U.top.name));
        } else if (A.place.kind !== "top") {
          for (let p = A.place; p; p = parentOf(p))
            if (p.kind === "block") {
              it.deps.push(actItem.get(p.A));
              break;
            }
        }
        it.startDeps = A.opts.has("awaiting") ? depItems(refsOf(A.opts.get("awaiting")), it) : [];
        it.slotDeps = new Map();
        for (const [k, v] of A.slots) if (!A.cells.has(k)) it.slotDeps.set(k, depItems(refsOf(v), it));

      } else if (it.kind === "factory") {
        const F = it.F;
        if (merged.has(F)) it.deps = [];
        else {
          const skip = new Set(allFactories.filter((x) => merged.has(x)).map((x) => x.call));
          it.deps = depItems(refsOf(F.call, skip), it);
        }
      } else it.deps = depItems(refsOf(it.node), it);
      // matchAll's iterator clones its regexp when made: the regexp's lastIndex (a patch
      // the dump writes later) must be set before
      const m = it.kind === "def" && /^\$dump\.regexpIterator\(\$dump\.(\w+),/.exec(src(it.node.expression.right));
      if (m) {
        const patch = items.find((x) => x.kind === "stmt" && src(x.node).startsWith(`$dump.${m[1]}.lastIndex = `));
        if (patch) {
          patch.pure = true;
          it.deps.push(patch);
        }
      }
      it.deps = [...new Set(it.deps.filter(Boolean))];
    }
    // Deps of an activation that need its own closures go into its callback, and the
    // bindings that take them are declared after the callback.
    const actOf = new Map([...actItem].map(([a, it]) => [it, a]));
    function needs(item, target, seen = new Set()) {
      if (item === target) return true;
      if (seen.has(item)) return false;
      seen.add(item);
      return (item.deps ?? []).some((d) => needs(d, target, seen));
    }
    for (const it of items) {
      if (it.kind !== "act") continue;
      const cyc = new Set(it.deps.filter((d) => needs(d, it)));
      if (!cyc.size) continue;
      it.deps = it.deps.filter((d) => !cyc.has(d));
      it.A.afterGo = new Set();
      for (const [k, ds] of it.slotDeps) if (ds.some((d) => cyc.has(d) || needs(d, it))) it.A.afterGo.add(k);
      it.A.needsGo = true;
    }
    // Pull scheduling.
    const top = [];
    let target = top;
    const stack = []; // open activations: {item, saved target}
    const emitted = new Set();
    const index = new Map(items.map((x, i) => [x, i]));
    const pure = (it) => {
      if (it.kind === "factory" || it.kind === "decl" || it.pure) return true;
      if (it.kind !== "def") return false;
      const t = src(it.node);
      return !/Promise\.(all|allSettled|any|race)|\.then\b|\$dump\.pending|\.next\(\)|\$dump\.resume/.test(t);
    };
    const busy = new Set();
    function ensureDeps(deps, upto) {
      for (const d of deps) {
        if (emitted.has(d)) continue;
        if (busy.has(d)) refuse("cyclic dependencies");
        if (d.kind === "act") {
          open(d, upto);
          continue;
        }
        if (index.get(d) > upto && !pure(d)) refuse("reordering an effect");
        busy.add(d);
        ensureDeps(d.deps, upto);
        busy.delete(d);
        put1(d);
      }
    }
    function put1(it) {
      if (emitted.has(it)) return;
      emitted.add(it);
      target.push(it);
    }
    // Calls activation item `a` now, with a callback for what follows up to its place.
    function open(a, upto) {
      if (busy.has(a)) refuse("cyclic dependencies");
      busy.add(a);
      ensureDeps(a.deps, upto);
      busy.delete(a);
      if (emitted.has(a)) return;
      emitted.add(a);
      const entry = { kind: "call", A: a.A, callback: [] };
      target.push(entry);
      stack.push({ a, saved: target });
      target = entry.callback;
      stats.callbacks++;
    }
    items.forEach((it, i) => {
      if (it.kind === "act") {
        const s = stack.findIndex((x) => x.a === it);
        if (s >= 0) {
          if (s !== stack.length - 1) refuse("crossing callbacks");
          ensureDeps(it.startDeps, i);
          target = stack.pop().saved;
          return;
        }
        ensureDeps(it.deps, i);
        if (it.A.needsGo && !emitted.has(it)) refuse("a frame needs data made after it");
        ensureDeps(it.startDeps, i);
        if (emitted.has(it)) return;
        emitted.add(it);
        target.push({ kind: "call", A: it.A, callback: null });
        return;
      }
      ensureDeps(it.deps, i);
      put1(it);
    });
    if (stack.length) refuse("unclosed callback");
    return top;
  }

  // ---- emission -----------------------------------------------------------------------------
  function emit(plan) {
    const out = [];
    const hoisted = []; // `var x;` of global vars assigned in callbacks
    const used = new Set();
    for (const k of Object.keys(HELPERS)) if ((k.length === 1 && k !== "N") || text.includes(`$dump.${k}(`)) used.add(k);
    if (used.has("regexpIterator")) used.add("MA");
    if (acts.some((A) => A.siteKind === "for-await.next")) used.add("N");
    const body = [];
    function emitItems(list, depth) {
      const lines = [];
      for (const it of list) {
        if (it.kind === "call") {
          const A = it.A;
          if (it.callback) {
            A.go = `$dump.g${A.name}();`;
            const inner = emitItems(it.callback, depth + 1);
            lines.push(`$dump.g${A.name} = () => {\n${inner.join("\n")}\n};`);
          }
          lines.push(...callText(A));
          continue;
        }
        if (it.kind === "factory") {
          if (merged.has(it.F)) continue; // a shell at the top
          lines.push(`$dump.${it.name} = ${factoryText(it.F)};`);
          continue;
        }
        if (it.kind === "decl" && depth > 0) {
          const d = it.node;
          if (d.type !== "VariableDeclaration" || d.kind !== "var") refuse("a global lexical declaration inside a callback");
          for (const x of d.declarations) {
            if (x.id.type !== "Identifier") refuse("a global pattern inside a callback");
            hoisted.push(`var ${x.id.name};`);
            if (x.init) lines.push(`${x.id.name} = ${rw(x.init)};`);
          }
          continue;
        }
        if (it.kind === "stmt" && omittedTarget(it.node)) continue;
        if (it.kind === "def" && omittedDefs.has(it.name)) continue;
        if (it.skip) continue;
        lines.push(rw(it.node));
      }
      return lines;
    }
    body.push(...emitItems(plan, 0));
    const head = [
      '"djs-plain 1";',
      `globalThis.$dump = { ${[...used].map((k) => `${k}: ${HELPERS[k]}`).join(",\n  ")} };`,
    ];
    for (const [path, ref] of builtins) head.push(`${ref} = ${builtinExpr(path)};`);
    for (const F of merged) if (F.parent === null) head.push(`$dump.${F.name} = {};`);
    for (const f of fnDecls) head.push(src(f));
    head.push(...hoisted);
    return [...head, ...body, `try { ${rw(handle)} } finally { delete globalThis.$dump; }`, ""].join("\n");
  }
  // The origin text as pieces: spans of K's own text ([start, length]) where K copied f
  // verbatim, and strings for the rest (the header, skeletons, loop heads, the site's
  // statement as it was), so the text a continuation already holds is not written twice.
  function originPieces(ftext, k) {
    const segs = [];
    for (let i = 0; i < k.segs.length; i += 3) segs.push([k.segs[i], k.segs[i + 1], k.segs[i + 2]]);
    const out = [];
    let lit = "";
    let p = 0;
    while (p < ftext.length) {
      let best = null;
      for (const [ks, fs, len] of segs) if (fs <= p && p < fs + len && (!best || fs + len > best[1] + best[2])) best = [ks, fs, len];
      if (best && best[1] + best[2] - p >= 8) {
        if (lit) out.push(lit), (lit = "");
        const len = best[1] + best[2] - p;
        out.push([best[0] + (p - best[1]), len]);
        p += len;
      } else lit += ftext[p++];
    }
    if (lit) out.push(lit);
    return out;
  }
  function omittedTarget(st) {
    const e = st.expression;
    if (e?.type !== "CallExpression" || src(e.callee) !== "Object.defineProperty") return false;
    const ch = memberChain(e.arguments[0]);
    if (ch?.length === 1 && omittedDefs.has(ch[0])) return true;
    if (!ch || ch.length !== 2 || !factories.has(ch[0])) return false;
    const r = findKey(factories.get(ch[0]), ch[1]);
    return r && r[0].omit.has(ch[1]);
  }
  function kref(A) {
    return A.f.home ? `$dump.${A.f.home.top.name}.$K${A.name}` : `$dump.K${A.name}`;
  }
  function callText(A) {
    const lines = [];
    const k = ktext(A);
    for (const [name, ref] of A.pre ?? []) lines.push(`${ref} = ${rw(A.slots.get(name))};`);
    if (!A.f.home) lines.push(`$dump.K${A.name} = ${k.text};`);
    const fref = A.f.ref && !A.f.omitKey && !A.f.omitDef ? A.f.ref : opts.pieces === false ? JSON.stringify(A.f.text) : JSON.stringify(originPieces(A.f.text, k));
    lines.push(`Object.defineProperty(${kref(A)}, "$djs", { value: { f: ${fref}, at: ${JSON.stringify(A.at)}, segs: [${k.segs.join(",")}], scaf: ${JSON.stringify(k.scaf)}${k.cmap.length ? "" : `, temps: ${JSON.stringify(k.temps)}`}, cmap: ${JSON.stringify(k.cmap)} } });`);
    const self = A.opts.has("this") ? rw(A.opts.get("this")) : "undefined";
    if (A.fn.generator) {
      if (A.f.ref && !A.f.omitKey && !A.f.omitDef) lines.push(`${kref(A)}.prototype = ${A.f.ref}.prototype;`);
      lines.push(`$dump.${A.name} = ${kref(A)}.call(${self});`);
      // primed: at a yield it runs to the placeholder yield; at an await the request being
      // served starts it (the first queued request)
      const q = A.queue ?? [];
      if (A.siteKind === "yield") lines.push(`$dump.${A.name}.next();`);
      else if (!A.fn.async || !q.length) refuse("a generator at an await without a request");
      q.forEach((r, i) => {
        const call = i === 0 && A.siteKind !== "yield" ? "next" : r.get("type").value;
        lines.push(`${rw(r.get("promise"))} = $dump.${A.name}.${call}(${i === 0 && A.siteKind !== "yield" ? "" : rw(r.get("value"))});`);
      });
    } else if (A.fkind === "arrow") lines.push(`$dump.${A.name} = (0, ${kref(A)})();`);
    else lines.push(`$dump.${A.name} = ${kref(A)}.call(${self});`);
    return lines;
  }
  function ktext(A) {
    if (ktexts.has(A)) return ktexts.get(A);
    A.blocksEmitted = 0;
    const g = gen(ctxFor(A, false));
    const unused = [...A.slots.keys()].filter((k) => !g.used.has(k) && !k.includes("@") === false && !A.cells.has(k));
    for (const k of A.slots.keys()) if (!g.used.has(k) && k.includes("@") && /[.-]/.test(k)) refuse(`unused slot ${k}`);
    const fl = flatten(g.parts, A.f.text, () => [...g.scaf, ...g.cmap]);
    const r = { text: fl.text, segs: fl.segs, scaf: g.scaf, temps: g.tempNames, cmap: g.cmap };
    ktexts.set(A, r);
    return r;
  }
  // An unmerged factory's text: merged subs are left out (their code moved into
  // continuations), continuations of functions it holds are added.
  function factoryText(F) {
    const lines = ["(() => {"];
    for (const sub of F.subs) if (!merged.has(sub)) lines.push(`const ${sub.name} = ${factoryText(sub)};`);
    for (const v of F.vars) if (!v.tdz) lines.push(`let ${v.name}${v.init ? ` = ${rw(v.init)}` : ""};`);
    const props = [];
    for (const sub of F.subs) if (!merged.has(sub) && F.spreads.includes(sub.name)) props.push(`...${sub.name}`);
    for (const [key, p] of F.props) if (!F.omit.has(key)) props.push(rw(p));
    for (const A of F.ks) props.push(`$K${A.name}: ${ktext(A).text}`);
    lines.push(`return { ${props.join(", ")} };`);
    for (const v of F.vars) if (v.tdz) lines.push(`let ${v.name};`);
    lines.push("})()");
    return lines.join("\n");
  }
  // What the dump puts in a place (a block of a continuation, or a dead scope group).
  function emitPlace(pl) {
    const lines = [];
    const assign = new Map(); // top name -> [prop texts]
    const flush = () => {
      for (const [t, ps] of assign) lines.push(`Object.assign($dump.${t}, { ${ps.join(", ")} });`);
      assign.clear();
    };
    for (const x of placed.get(pl) ?? []) {
      const t = x.F?.top.name ?? x.A.f.home.top.name;
      if (x.kind === "prop" || x.kind === "k") {
        if (!assign.has(t)) assign.set(t, []);
        assign.get(t).push(x.kind === "prop" ? rw(x.node) : `$K${x.A.name}: ${ktext(x.A).text}`);
      } else if (x.kind === "sub") {
        flush();
        lines.push(`Object.assign($dump.${t}, ${factoryText(x.sub)});`);
      } else if (x.kind === "group") {
        flush();
        const label = `$djs_${x.F.name}`;
        const inner = [];
        for (const v of x.F.vars) if (!v.owner && !v.tdz) inner.push(`let ${v.name}${v.init ? ` = ${rw(v.init)}` : ""};`);
        inner.push(...emitPlace(x.G));
        const tdz = x.F.vars.filter((v) => !v.owner && v.tdz);
        if (tdz.length) lines.push(`${label}: {\n${inner.join("\n")}\nbreak ${label};\nlet ${tdz.map((v) => v.name).join(", ")};\n}`);
        else lines.push(`{\n${inner.join("\n")}\n}`);
      }
    }
    flush();
    return lines;
  }

  // ---- rewriting D expressions ----------------------------------------------------------
  function rw(node) {
    const reps = [];
    (function visit(n) {
      if (n.type === "CallExpression" && src(n.callee) === "$dump.builtin") {
        reps.push([n.start, n.end, builtinRef(n.arguments[0].value)]);
        return;
      }
      if (n.type === "CallExpression" && src(n.callee) === "$dump.pending") {
        // an open host call is made again: the host decides what a repeated call means
        // (deliver a recorded result, run it again); its state, if any, is the host's
        const name = n.arguments[1].value;
        if (!/^[\w$]+(\.[\w$]+)*$/.test(name)) refuse(`host function ${name}`);
        const args = n.arguments[2].elements.map((e) => rw(e)).join(", ");
        reps.push([n.start, n.end, `${name}(${args})`]);
        return;
      }
      for (const c of children(n)) visit(c);
    })(node);
    if (!reps.length) return src(node);
    let out = "";
    let at = node.start;
    for (const [s, e, t] of reps) {
      out += text.slice(at, s) + t;
      at = e;
    }
    return out + text.slice(at, node.end);
  }
  function builtinRef(path) {
    if (!builtins.has(path)) builtins.set(path, `$dump.b${builtins.size}`);
    return builtins.get(path);
  }
}
