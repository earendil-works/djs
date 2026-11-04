// 0188 prototype: rewrite a format 2 dump's `$dump.resume(fn, {...})` activations as
// residual continuations in plain JavaScript (variant "R" of
// docs/suspension-design-review.md). Not engine code: a scratch measurement.
//
// For an activation of `fn` stopped at site `at`, the continuation is a function of the
// same kind as `fn` (a method, so it has no self binding and can be named like `fn`)
// whose body is the rest of `fn`'s body from the site on: statements before the site are
// dropped, the bindings live there are declared from the dump's values, the statement
// holding the site is rewritten with the evaluated operands in place, and every loop
// around the site becomes a loop that runs the rest of the current iteration first.
// Nested functions in the rest are copied verbatim (their text and toString are kept).
//
// The dump is then: everything as before, except each `$dump.resume(...)` becomes
//   $dump.aN = $K.m.call(this, [values...])
// where $K is an object literal holding the continuation as method `m`, written where
// the activation's shared variables are in scope (inside the factory that made them).
//
// Usage: node residualize.mjs in.dump.js > out.dump.js
import * as acorn from "acorn";
import fs from "node:fs";

const text = fs.readFileSync(process.argv[2], "utf8");
const ast = acorn.parse(text, { ecmaVersion: "latest", sourceType: "script", ranges: true });
const src = (n) => text.slice(n.start, n.end);
class Refuse extends Error {}
const refuse = (m) => {
  process.stderr.write("refused: " + m + "\n");
  throw new Refuse(m);
};

// ---- walking -----------------------------------------------------------------------
function children(n) {
  const out = [];
  for (const k of Object.keys(n)) {
    if (k === "type" || k === "start" || k === "end" || k === "range") continue;
    const v = n[k];
    if (Array.isArray(v)) for (const c of v) c && typeof c.type === "string" && out.push(c);
    else if (v && typeof v.type === "string") out.push(v);
  }
  return out.sort((a, b) => a.start - b.start);
}
const isFn = (n) =>
  n.type === "FunctionDeclaration" ||
  n.type === "FunctionExpression" ||
  n.type === "ArrowFunctionExpression";

// The function nodes of the dump, by name (declarations, named expressions, properties).
const fns = [];
(function visit(n, propName) {
  if (isFn(n)) fns.push({ node: n, name: n.id?.name ?? propName });
  for (const c of children(n)) {
    let pn;
    if (n.type === "Property" && c === n.value && n.key.type === "Identifier") pn = n.key.name;
    if (n.type === "VariableDeclarator" && c === n.init && n.id.type === "Identifier") pn = n.id.name;
    visit(c, pn);
  }
})(ast);

// Sites of `fn` in source order, not inside nested functions.
function sites(fn) {
  const out = { await: [], yield: [] };
  (function visit(n) {
    if (n !== fn && isFn(n)) return;
    if (n.type === "AwaitExpression") out.await.push(n);
    if (n.type === "YieldExpression" && !n.delegate) out.yield.push(n);
    if (n.type === "ForOfStatement" && n.await) out.forAwait = true;
    for (const c of children(n)) visit(c);
  })(fn);
  // djs numbers sites by the position of the keyword: an await inside another's
  // operand comes after it.
  out.await.sort((a, b) => a.start - b.start);
  out.yield.sort((a, b) => a.start - b.start);
  return out;
}

function pathTo(root, target) {
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

// ---- the dump's resume calls ------------------------------------------------------------
function objProps(o) {
  const m = new Map();
  for (const p of o.properties) {
    const k = p.key.type === "Identifier" ? p.key.name : p.key.value;
    m.set(k, p.value);
  }
  return m;
}

// Names declared directly by a statement list (lexical) or hoisted to a function (var,
// function declarations).
function declaredBy(stmt) {
  const names = [];
  const pat = (p) => {
    if (!p) return;
    if (p.type === "Identifier") names.push(p.name);
    else if (p.type === "ObjectPattern") p.properties.forEach((q) => pat(q.type === "RestElement" ? q.argument : q.value));
    else if (p.type === "ArrayPattern") p.elements.forEach(pat);
    else if (p.type === "RestElement") pat(p.argument);
    else if (p.type === "AssignmentPattern") pat(p.left);
  };
  if (stmt.type === "VariableDeclaration") stmt.declarations.forEach((d) => pat(d.id));
  if (stmt.type === "FunctionDeclaration" || stmt.type === "ClassDeclaration") names.push(stmt.id.name);
  return { names, kind: stmt.type === "VariableDeclaration" ? stmt.kind : stmt.type === "FunctionDeclaration" ? "function" : "let", pat };
}
function patNames(p) {
  const d = declaredBy({ type: "VariableDeclaration", kind: "let", declarations: [{ id: p }] });
  return d.names;
}

// ---- the residual -------------------------------------------------------------------
function residual(fn, site, res, g) {
  const bindings = res.get("bindings") ? objProps(res.get("bindings")) : new Map();
  const operands = res.get("operands") ? res.get("operands").elements.map((e) => [e.elements[0].value, src(e.elements[1])]) : [];
  const loops = res.get("loops") ? res.get("loops").elements.map((e) => objProps(e)) : [];
  const values = []; // the saved values, passed to the continuation as `$s`
  const val = (exprText) => {
    values.push(exprText);
    return `$t${g}_${values.length - 1}`;
  };
  const bind = new Map();
  for (const [k, v] of bindings) bind.set(k, src(v));
  const cellNames = new Set([...bind].filter(([, v]) => /^\$dump\.s\d+\.\$c\d+$/.test(v)).map(([k]) => k));
  const tdzCells = [];
  const declared = new Set();
  const decl = (name, kind) => {
    if (!bind.has(name) || declared.has(name)) return "";
    declared.add(name);
    const v = bind.get(name);
    if (v === "$dump.TDZ") return ""; // declared by the rest of the code
    if (name === "arguments") return `const $arguments = ${val(v)};`;
    // A cell: the variable of the factory the continuation is written in.
    if (/^\$dump\.s\d+\.\$c\d+$/.test(v)) return "";
    return `${kind === "const" ? "const" : "let"} ${name} = ${val(v)};`;
  };
  let opIdx = 0;
  const op = (role) => {
    const o = operands[opIdx];
    if (!o || (role && !o[0].startsWith(role))) refuse(`operand ${opIdx}: want ${role}, have ${o?.[0]}`);
    opIdx++;
    return val(o[1]);
  };
  let loopIdx = 0;
  let tmp = 0;
  const path = pathTo(fn.body, site);
  const kind = fn.async && fn.generator ? "async *" : fn.async ? "async " : fn.generator ? "*" : "";
  let usesArguments = false;

  // The rest of the code `n` (verbatim except `arguments` -> `$arguments`).
  function rest(n) {
    let s = src(n);
    const repl = [];
    (function visit(m) {
      if (m !== n && isFn(m) && m.type !== "ArrowFunctionExpression") return;
      if (m.type === "Identifier" && m.name === "arguments") repl.push(m);
      for (const c of children(m)) visit(c);
    })(n);
    for (const m of repl.reverse()) {
      usesArguments = true;
      s = s.slice(0, m.start - n.start) + "$arguments" + s.slice(m.end - n.start);
    }
    return s;
  }

  // The expression `n` on the path, from the site up: evaluated children before the
  // path child come from operands, those after are verbatim.
  function expr(i) {
    const n = path[i];
    const next = path[i + 1];
    if (n === site) {
      if (n.type === "AwaitExpression") return `(await ${val(src(res.get("awaiting")))})`;
      return "(yield)"; // a yield's operand was yielded before the dump
    }
    switch (n.type) {
      case "AwaitExpression":
        return `(await ${expr(i + 1)})`;
      case "CallExpression": {
        if (n.optional) refuse("optional call");
        if (next === n.callee) {
          return `${expr(i + 1)}(${n.arguments.map(rest).join(", ")})`;
        }
        if (n.callee.type === "Super" || n.callee.type === "Import") refuse("super/import call");
        const callee = op("call.callee");
        const self = op("call.this");
        const args = [];
        let seen = false;
        // With a spread before the site, the arguments so far are one array (call.args).
        const spreadAt = n.arguments.findIndex((a) => a.type === "SpreadElement");
        const site = n.arguments.indexOf(next);
        if (spreadAt >= 0 && spreadAt < site) {
          args.push(`...${op("call.args")}`);
          args.push(expr(i + 1));
          for (const a of n.arguments.slice(site + 1)) args.push(rest(a));
          return `$apply(${callee}, ${self}, [${args.join(", ")}])`;
        }
        for (const a of n.arguments) {
          if (a === next) {
            args.push(expr(i + 1));
            seen = true;
          } else args.push(seen ? rest(a) : op("call.arg"));
        }
        // Reflect.apply, as the dump found it at load: a call with the evaluated callee
        // and this value. Caveat: it is a frame of its own in stack traces.
        return `$apply(${callee}, ${self}, [${args.join(", ")}])`;
      }
      case "BinaryExpression":
        if (next === n.left) return `(${expr(i + 1)} ${n.operator} ${rest(n.right)})`;
        return `(${op("binary.left")} ${n.operator} ${expr(i + 1)})`;
      case "LogicalExpression":
        if (next === n.left) return `(${expr(i + 1)} ${n.operator} ${rest(n.right)})`;
        return expr(i + 1); // the left side decided to evaluate the right
      case "TemplateLiteral": {
        if (n.type === "TaggedTemplateExpression") refuse("tagged template");
        // Parts evaluated so far are strings (ToString applied): they are substitutions
        // of a template again, so the rest keeps template semantics.
        let out = "`";
        let seen = false;
        n.quasis.forEach((q, k) => {
          if (!seen) out += "${" + op("template.part") + "}";
          else out += q.value.raw;
          if (k < n.expressions.length) {
            const e = n.expressions[k];
            if (e === next) {
              out += "${" + expr(i + 1) + "}";
              seen = true;
            } else if (seen) out += "${" + rest(e) + "}";
            else out += "${" + op("template.part") + "}";
          }
        });
        return out + "`";
      }
      case "ArrayExpression": {
        const els = [];
        let seen = false;
        for (const e of n.elements) {
          if (e === next) {
            els.push(expr(i + 1));
            seen = true;
          } else if (seen) els.push(e ? rest(e) : "");
          else els.push(op("array.element"));
        }
        return `[${els.join(", ")}]`;
      }
      case "MemberExpression":
        if (next === n.object) return `${expr(i + 1)}${n.computed ? `[${rest(n.property)}]` : `.${n.property.name}`}`;
        return `${op("member.object")}[${expr(i + 1)}]`;
      case "AssignmentExpression": {
        if (next !== n.right) refuse("site in an assignment target");
        if (n.operator === "&&=" || n.operator === "||=" || n.operator === "??=") return `(${src(n.left)} = ${expr(i + 1)})`;
        let target;
        if (n.left.type === "Identifier") target = n.left.name;
        else if (n.left.type === "MemberExpression" && n.left.object.type !== "Super") {
          const o = op("assign.object");
          // A computed key the compiler kept (else it read a binding that cannot change).
          const k = n.left.computed ? (operands[opIdx]?.[0] === "assign.key" ? op("assign.key") : rest(n.left.property)) : null;
          target = k === null ? `${o}.${n.left.property.name}` : `${o}[${k}]`;
        } else refuse("assignment to a pattern or super");
        if (n.operator === "=") return `(${target} = ${expr(i + 1)})`;
        return `(${target} = ${op("assign.old")} ${n.operator.slice(0, -1)} ${expr(i + 1)})`;
      }
      case "ObjectExpression": {
        // The object made so far; the rest of the literal is defined on it in order.
        const o = op("object.literal");
        const out = [];
        let seen = false;
        const define = (k, v) => `$define(${o}, ${k}, ${v})`;
        for (const p of n.properties) {
          if (!seen && p !== next) continue;
          if (p.type === "SpreadElement") {
            if (p === next) {
              seen = true;
              out.push(`$copy(${o}, ${expr(i + 2)})`);
            } else out.push(`$copy(${o}, ${rest(p.argument)})`);
            continue;
          }
          if (p.kind !== "init" || p.method) refuse("accessor or method after the site");
          if (p.key.type === "Identifier" && p.key.name === "__proto__" && !p.computed && !p.shorthand) refuse("__proto__");
          const key = p.computed ? (p === next && operands[opIdx]?.[0] === "object.key" ? op("object.key") : rest(p.key)) : JSON.stringify(p.key.name ?? p.key.value);
          if (p === next) {
            seen = true;
            if (path[i + 2] !== p.value) refuse("site in a computed key");
            out.push(define(key, expr(i + 2)));
          } else out.push(define(key, rest(p.value)));
        }
        return `(${out.join(", ")}, ${o})`;
      }
      case "NewExpression": {
        if (next === n.callee || n.arguments.some((a) => a.type === "SpreadElement")) refuse("new with spread or a site in its callee");
        const callee = op("new.callee");
        const args = [];
        let seen = false;
        for (const a of n.arguments) {
          if (a === next) {
            args.push(expr(i + 1));
            seen = true;
          } else args.push(seen ? rest(a) : op("new.arg"));
        }
        return `$construct(${callee}, [${args.join(", ")}])`;
      }
      case "SpreadElement":
        return `...${expr(i + 1)}`;
      case "YieldExpression":
        return `(yield ${expr(i + 1)})`;
      case "UnaryExpression":
        return `(${n.operator} ${expr(i + 1)})`;
      case "ConditionalExpression":
        if (next === n.test) return `(${expr(i + 1)} ? ${rest(n.consequent)} : ${rest(n.alternate)})`;
        return expr(i + 1);
      case "ParenthesizedExpression":
        return expr(i + 1);
      default:
        refuse(`expression ${n.type}`);
    }
  }

  // A statement list from child `j` (which holds the site) on.
  function list(stmts, j, i) {
    const out = [];
    for (let k = 0; k < stmts.length; k++) {
      const s = stmts[k];
      const d = declaredBy(s);
      if (k < j || s.type === "FunctionDeclaration") {
        // Ran already (or hoisted): its bindings come from the dump.
        for (const name of d.names) out.push(decl(name, d.kind));
      }
    }
    out.push(stmt(i));
    for (let k = j + 1; k < stmts.length; k++) {
      const s = stmts[k];
      if (s.type === "FunctionDeclaration") continue;
      if (s.type === "VariableDeclaration" && declaredBy(s).names.some((m) => cellNames.has(m))) {
        // A variable a closure shares, still in its TDZ: the factory's variable is
        // initialized here instead (lossy: it reads as undefined before, and a const
        // can be assigned).
        if (s.declarations.some((d) => d.id.type !== "Identifier")) refuse("pattern over a shared TDZ variable");
        tdzCells.push(...declaredBy(s).names);
        out.push(s.declarations.map((d) => `${d.id.name} = ${d.init ? rest(d.init) : "undefined"};`).join(" "));
        continue;
      }
      out.push(rest(s));
    }
    return out.filter(Boolean).join("\n");
  }

  // The statement path[i] holding the site.
  function stmt(i) {
    const n = path[i];
    const next = path[i + 1];
    switch (n.type) {
      case "BlockStatement":
        return `{\n${list(n.body, n.body.indexOf(next), i + 1)}\n}`;
      case "ExpressionStatement":
        return `${expr(i + 1)};`;
      case "ReturnStatement":
        return `return ${expr(i + 1)};`;
      case "ThrowStatement":
        return `throw ${expr(i + 1)};`;
      case "VariableDeclaration": {
        const parts = [];
        let seen = false;
        for (const d of n.declarations) {
          if (d === next) {
            if (next !== d || path[i + 2] !== d.init) refuse("site in a pattern");
            parts.push(`${src(d.id)} = ${expr(i + 2)}`);
            seen = true;
          } else if (seen) parts.push(rest(d));
          else patNames(d.id).forEach((m) => parts.push(`${m} = ${val(bind.get(m))}`));
        }
        return `${n.kind} ${parts.join(", ")};`;
      }
      case "IfStatement":
        if (next === n.test) return `if (${expr(i + 1)}) ${rest(n.consequent)}${n.alternate ? ` else ${rest(n.alternate)}` : ""}`;
        return stmt(i + 1);
      case "LabeledStatement":
        return `${n.label.name}: ${stmt(i + 1)}`;
      case "ForOfStatement": {
        if (next !== n.body) refuse("site in a for-of head");
        if (n.await) refuse("for await");
        const loop = loops[loopIdx++] ?? refuse("missing loop record");
        // The loop runs the rest of this iteration first, then the iterator's next steps
        // in the original body; `$loop` (a loader primitive in a real design) is an
        // iterable over the saved iterator record whose first step yields a placeholder.
        const f = `$f${g}_${tmp++}`;
        const left =
          n.left.type === "VariableDeclaration"
            ? `${n.left.kind} ${src(n.left.declarations[0].id)} = $v${g};`
            : `${src(n.left)} = $v${g};`;
        const names = n.left.type === "VariableDeclaration" ? patNames(n.left.declarations[0].id) : [];
        const own = names.map((m) => decl(m, n.left.kind)).join("\n");
        return `{ let ${f} = true; for (const $v${g} of $loop(${val(src(loop.get("iterator")))}, ${val(src(loop.get("next")))})) { if (${f}) { ${f} = false; {\n${own}\n${body(n.body, i + 1)}\n} } else { ${left} ${rest(n.body)} } } }`;
      }
      case "DoWhileStatement": {
        if (next !== n.body) refuse("site in a do-while test");
        const f = `$f${g}_${tmp++}`;
        return `{ let ${f} = true; do { if (${f}) { ${f} = false; ${body(n.body, i + 1)} } else ${rest(n.body)} } while (${rest(n.test)}); }`;
      }
      case "WhileStatement": {
        if (next !== n.body) refuse("site in a while test");
        const f = `$f${g}_${tmp++}`;
        return `{ let ${f} = true; while (${f} || (${rest(n.test)})) { if (${f}) { ${f} = false; ${body(n.body, i + 1)} } else ${rest(n.body)} } }`;
      }
      case "ForStatement": {
        if (next !== n.body) refuse("site in a for head");
        const f = `$f${g}_${tmp++}`;
        let init = "";
        if (n.init?.type === "VariableDeclaration") {
          init = `${n.init.kind} ${n.init.declarations.flatMap((d) => patNames(d.id)).map((m) => `${m} = ${val(bind.get(m))}`).join(", ")}`;
          n.init.declarations.flatMap((d) => patNames(d.id)).forEach((m) => declared.add(m));
        }
        return `{ let ${f} = true; for (${init}; ${f} || (${n.test ? rest(n.test) : "true"}); ${n.update ? rest(n.update) : ""}) { if (${f}) { ${f} = false; ${body(n.body, i + 1)} } else ${rest(n.body)} } }`;
      }
      case "TryStatement": {
        if (next === n.block) {
          return `try ${stmt(i + 1)}${n.handler ? ` catch${n.handler.param ? ` (${src(n.handler.param)})` : ""} ${rest(n.handler.body)}` : ""}${n.finalizer ? ` finally ${rest(n.finalizer)}` : ""}`;
        }
        if (next === n.handler) {
          const names = n.handler.param ? patNames(n.handler.param) : [];
          const own = names.map((m) => decl(m, "let")).join("\n");
          const h = `{\n${own}\n${stmt(i + 2)}\n}`;
          return n.finalizer ? `try ${h} finally ${rest(n.finalizer)}` : h;
        }
        // In the finally block: the rest of it, then the completion that entered it
        // (`finally.value`, `finally.return`: the index of the gosub, in emission order).
        const v = op("finally.value");
        const idx = Number(operands[opIdx - 1 + 1]?.[1]);
        opIdx++; // finally.return
        const exits = finallyExits(n);
        const exit = exits[idx] ?? refuse(`finally gosub ${idx}`);
        const done =
          exit.kind === "normal" ? "" : exit.kind === "throw" ? `throw ${v};` : exit.kind === "return" ? `return ${v};` : `${exit.kind} ${exit.label ?? ""};`;
        return `{ ${stmt(i + 1)} ${done} }`;
      }
      case "SwitchStatement": {
        const m = n.cases.findIndex((c) => c.start <= site.start && site.end <= c.end);
        const c = n.cases[m];
        const before = n.cases.slice(0, m);
        // Lexical declarations of earlier cases are in the same scope: not handled.
        if (before.some((b) => b.consequent.some((x) => declaredBy(x).names.length))) refuse("declarations in an earlier case");
        if (c.test && c.test.start <= site.start && site.end <= c.test.end) {
          // A case test: the discriminant was kept (twice) for the comparison; the
          // earlier cases did not match, so the switch starts at this one (unless a
          // default before it is where an unmatched switch goes).
          if (before.some((b) => !b.test)) refuse("default before the case being tested");
          const d = op("switch.discriminant");
          op("switch.discriminant");
          const ci = path.indexOf(c);
          const cases = n.cases.slice(m + 1).map(rest).join("\n");
          return `switch (${d}) {\ncase ${expr(ci + 1)}: ${c.consequent.map(rest).join("\n")}\n${cases}\n}`;
        }
        // In a case's statements: the rest of them, then the later cases' statements
        // (falling through), in one switch so `break` still leaves it.
        const ci = path.indexOf(c);
        const later = n.cases.slice(m + 1).flatMap((x) => x.consequent).map(rest).join("\n");
        return `switch (0) { case 0: ${list(c.consequent, c.consequent.indexOf(path[ci + 1]), ci + 1)}\n${later} }`;
      }
      default:
        refuse(`statement ${n.type}`);
    }
  }
  // A loop body holding the site: a block or a single statement.
  function body(b, i) {
    return b.type === "BlockStatement" ? stmt(i) : stmt(i);
  }
  // The ways into a finally block, in djs's gosub emission order: jumps out of the try
  // block (and catch block) as they appear, the normal exits, the exception path last.
  function finallyExits(t) {
    const exits = [];
    const jumps = (n, inLoop) => {
      if (isFn(n)) return;
      // A generator's yield in the try block: its return() path enters the finally.
      if (n.type === "ReturnStatement" || (n.type === "YieldExpression" && fn.generator)) exits.push({ kind: "return" });
      if ((n.type === "BreakStatement" || n.type === "ContinueStatement") && !(inLoop && !n.label))
        exits.push({ kind: n.type === "BreakStatement" ? "break" : "continue", label: n.label?.name });
      const loop = /^(For|ForIn|ForOf|While|DoWhile)Statement$|^SwitchStatement$/.test(n.type);
      for (const c of children(n)) jumps(c, inLoop || loop);
    };
    jumps(t.block, false);
    if (t.handler) jumps(t.handler.body, false);
    exits.push({ kind: "normal" });
    if (t.handler) exits.push({ kind: "normal" });
    exits.push({ kind: "throw" });
    return exits;
  }

  // Parameters and the function body's own declarations.
  const params = fn.params.flatMap(patNames);
  const head = [...params, "arguments"].map((p) => decl(p, "let")).filter(Boolean);
  let bodyText = fn.body.type === "BlockStatement" ? list(fn.body.body, fn.body.body.indexOf(path[1]), 1) : `return ${expr(0)};`;
  // var declarations anywhere in the function are function-level.
  if (bind.has("arguments")) usesArguments = true;
  if (opIdx !== operands.length) refuse(`unused operands ${operands.slice(opIdx).map((o) => o[0])}`);
  if (loopIdx !== loops.length) refuse("unused loops");
  const name = fns.find((f) => f.node === fn)?.name ?? "anonymous";
  // An arrow's continuation is an arrow (its this, arguments and new.target are the
  // scope's, as they are the original's); others are methods, called with the this value.
  // The saved values become locals and the array is dropped, so a later dump of the
  // continuation does not carry it.
  const temps = values.length ? `const ${values.map((_, k) => `$t${g}_${k} = $s${g}[${k}]`).join(", ")};\n$s${g} = undefined;` : "";
  head.unshift(temps);
  const method =
    fn.type === "ArrowFunctionExpression"
      ? `${JSON.stringify(name)}: async ($s${g}) => {\n${head.join("\n")}\n${bodyText}\n}`
      : `${kind}${JSON.stringify(name)}($s${g}) {\n${head.join("\n")}\n${bodyText}\n}`;
  return { method, name, values, tdzCells, cells: [...bind.values()].filter((v) => /^\$dump\.s\d+\.\$c\d+$/.test(v)) };
}

// ---- rewriting the dump -----------------------------------------------------------------
const edits = [];
// Continuation holders of an earlier generation are globals of this dump: new ones count on.
let conts = Math.max(-1, ...[...text.matchAll(/\$K(\d+)\b/g)].map((m) => Number(m[1]))) + 1;
const report = [];
for (const st of ast.body) {
  if (st.type !== "ExpressionStatement" || st.expression.type !== "AssignmentExpression") continue;
  const call = st.expression.right;
  if (call.type !== "CallExpression" || src(call.callee) !== "$dump.resume") continue;
  const fnRef = src(call.arguments[0]);
  const res = objProps(call.arguments[1]);
  const at = res.get("at").value;
  const name = fnRef.split(".").pop();
  // The function the reference names: inline, a factory's property, or the last function
  // of that name defined before this statement (good enough for the prototype).
  let fn;
  if (isFn(call.arguments[0])) fn = call.arguments[0];
  else if (/^\$dump\.s\d+\./.test(fnRef)) {
    const home = fnRef.split(".").slice(0, 2).join(".");
    const def = ast.body.find((x) => x.type === "ExpressionStatement" && x.expression.left && src(x.expression.left) === home);
    fn = fns.find((f) => f.name === name && f.node.start > def.start && f.node.end < def.end)?.node;
  } else if (/^\$dump\.\w+$/.test(fnRef)) {
    const def = ast.body.find((x) => x.type === "ExpressionStatement" && x.expression.left && src(x.expression.left) === fnRef);
    if (def && isFn(def.expression.right)) fn = def.expression.right;
    // `$dump.fn({ m() {} })`: a method.
    const r = def?.expression.right;
    if (r?.type === "CallExpression" && src(r.callee) === "$dump.fn") fn = r.arguments[0].properties[0].value;
  }
  fn ??= fns.filter((f) => f.name === name && f.node.end < st.start).at(-1)?.node ?? refuse(`no function ${fnRef}`);
  const [k, nth] = at.split("#");
  const s = sites(fn)[k]?.[Number(nth) - 1] ?? refuse(`no site ${at}`);
  const gid = conts++;
  const r = residual(fn, s, res, gid);
  const id = `$K${gid}`;
  const self = res.get("this") ? src(res.get("this")) : "undefined";
  let callText = isFn(fn) && fn.type === "ArrowFunctionExpression"
    ? `${id}[${JSON.stringify(r.name)}]([${r.values.join(", ")}])`
    : `${id}[${JSON.stringify(r.name)}].call(${self}, [${r.values.join(", ")}])`;
  if (k === "yield") callText = `$prime(${callText}, ${fnRef})`;
  if (res.get("queue")) {
    // An async generator's requests are made again on the continuation, in order: their
    // promises are the ones data holds.
    const q = res.get("queue").elements.map(objProps);
    let again = "";
    for (const e of q) {
      const p = src(e.get("promise"));
      // Remove the placeholder definition, re-make the request after the activation.
      const def = ast.body.find((x) => x.type === "ExpressionStatement" && x.expression.left && src(x.expression.left) === p);
      edits.push({ start: def.start, end: def.end, text: "" });
      again += `\n${p} = ${src(st.expression.left)}.${e.get("type").value}(${src(e.get("value"))});`;
    }
    edits.push({ start: st.end, end: st.end, text: again });
  }
  // Where the continuation goes: in the factory of its cells, else at the top level.
  // The continuation's free names must resolve as `fn`'s do: it is written in the factory
  // that wrote `fn`, and its frame's shared variables must be that factory's too.
  const fnHome = /^\$dump\.s\d+\./.test(fnRef) ? fnRef.split(".").slice(0, 2).join(".") : null;
  const cellHome = r.cells.length ? r.cells[0].split(".").slice(0, 2).join(".") : null;
  if (fnHome && cellHome && fnHome !== cellHome) refuse("function and cells in different factories");
  if (r.cells.some((c) => !c.startsWith(cellHome + "."))) refuse("cells of several factories");
  const where = fnHome ?? cellHome;
  if (where) {
    const def = ast.body.find((x) => x.type === "ExpressionStatement" && x.expression.left && src(x.expression.left) === where);
    const fbody = def.expression.right.callee.body.body;
    const ret = fbody.find((x) => x.type === "ReturnStatement");
    // Shared variables in their TDZ are declared after the factory's return; the
    // continuation initializes them, so they move before it.
    for (const x of fbody.slice(fbody.indexOf(ret) + 1)) {
      if (x.type === "VariableDeclaration" && declaredBy(x).names.some((m) => r.tdzCells.includes(m))) {
        edits.push({ start: x.start, end: x.end, text: "" });
        edits.push({ start: ret.start, end: ret.start, text: `let ${declaredBy(x).names.join(", ")};\n` });
      }
    }
    edits.push({ start: ret.start, end: ret.start, text: `const ${id} = { ${r.method} };\n` });
    const obj = ret.argument;
    edits.push({ start: obj.end - 1, end: obj.end - 1, text: `, ${id}` });
    callText = callText.replace(id, `${where}.${id}`);
  } else {
    edits.push({ start: st.start, end: st.start, text: `const ${id} = { ${r.method} };\n` });
  }
  edits.push({ start: call.start, end: call.end, text: callText });
  report.push(`${fnRef} ${at}: ${r.values.length} values`);
}
// The helpers a real design would make loader primitives that outlive the load.
const helpers = `const $apply = Reflect.apply;
const $loop = (it, next) => { let first = true; const r = { next() { if (first) { first = false; return { value: undefined, done: false }; } return $apply(next, it, []); }, return() { const m = it.return; return m === undefined || m === null ? {} : $apply(m, it, []); } }; return { [Symbol.iterator]() { return r; } }; };
const $define = (o, k, v) => Object.defineProperty(o, k, { value: v, writable: true, enumerable: true, configurable: true });
const $copy = (o, from) => Object.assign(o, from);
const $construct = Reflect.construct;
const $prime = (g, fn) => (g.next(), Object.setPrototypeOf(g, fn.prototype), g);
`;
const firstStmt = ast.body[1];
// A dump of a run that resumed continuations has the helpers as its own globals already.
if (!text.includes("const $apply = ")) edits.push({ start: firstStmt.start, end: firstStmt.start, text: helpers });
edits.sort((a, b) => b.start - a.start || b.end - a.end);
let out = text;
for (const e of edits) out = out.slice(0, e.start) + e.text + out.slice(e.end);
process.stdout.write(out);
process.stderr.write(report.join("\n") + (report.length ? "\n" : ""));
