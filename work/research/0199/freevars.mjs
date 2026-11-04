// Free variable names of a function/class/expression (acorn AST): names it reads or
// writes that it does not declare itself. Scope-aware enough for placement decisions.
import { children, patNames } from "./plain.mjs";

const FN = new Set(["FunctionDeclaration", "FunctionExpression", "ArrowFunctionExpression"]);

export function freeVars(node) {
  const free = new Set();
  // scopes: arrays of Sets
  function hoist(body, set) {
    // var and function declarations of a function body (not nested functions)
    (function v(n, top) {
      if (FN.has(n.type) && !top) {
        if (n.type === "FunctionDeclaration" && n.id) set.add(n.id.name);
        return;
      }
      if (n.type === "VariableDeclaration" && n.kind === "var") n.declarations.forEach((d) => patNames(d.id).forEach((x) => set.add(x)));
      if (n.type === "ClassExpression" || n.type === "ClassDeclaration") {
        if (n.type === "ClassDeclaration") return; // block-scoped
      }
      for (const c of children(n)) v(c, false);
    })(body, true);
  }
  function blockDecls(stmts, set) {
    for (const s of stmts) {
      if (s.type === "VariableDeclaration" && s.kind !== "var") s.declarations.forEach((d) => patNames(d.id).forEach((x) => set.add(x)));
      if (s.type === "ClassDeclaration") set.add(s.id.name);
      if (s.type === "FunctionDeclaration") set.add(s.id.name);
    }
  }
  const declared = (scopes, name) => scopes.some((s) => s.has(name));
  function walk(n, scopes, parent, key) {
    if (!n) return;
    switch (n.type) {
      case "Identifier": {
        // references only: not property keys, not member properties, not labels
        if (parent) {
          if (parent.type === "MemberExpression" && key === "property" && !parent.computed) return;
          if ((parent.type === "Property" || parent.type === "MethodDefinition" || parent.type === "PropertyDefinition") && key === "key" && !parent.computed) {
            if (parent.type === "Property" && parent.shorthand) break; // `{x}` reads x
            return;
          }
          if (/^(Labeled|Break|Continue)Statement$/.test(parent.type)) return;
          if (parent.type === "MetaProperty") return;
          if ((parent.type === "ExportSpecifier" || parent.type === "ImportSpecifier")) return;
        }
        if (!declared(scopes, n.name)) free.add(n.name);
        return;
      }
      case "FunctionDeclaration":
      case "FunctionExpression":
      case "ArrowFunctionExpression": {
        const s = new Set();
        if (n.type === "FunctionExpression" && n.id) s.add(n.id.name);
        if (n.type !== "ArrowFunctionExpression") s.add("arguments").add("this");
        n.params.forEach((p) => patNames(p).forEach((x) => s.add(x)));
        if (n.body.type === "BlockStatement") {
          hoist(n.body, s);
          blockDecls(n.body.body, s);
        }
        const inner = [...scopes, s];
        n.params.forEach((p) => walkPattern(p, inner));
        if (n.body.type === "BlockStatement") n.body.body.forEach((c) => walk(c, inner, n.body, "body"));
        else walk(n.body, inner, n, "body");
        return;
      }
      case "ClassDeclaration":
      case "ClassExpression": {
        const s = new Set();
        if (n.id) s.add(n.id.name);
        const inner = [...scopes, s];
        if (n.superClass) walk(n.superClass, inner, n, "superClass");
        for (const m of n.body.body) {
          if (m.computed) walk(m.key, inner, m, "key");
          if (m.value) walk(m.value, inner, m, "value");
          if (m.type === "StaticBlock") {
            const b = new Set();
            blockDecls(m.body, b);
            m.body.forEach((c) => walk(c, [...inner, b], m, "body"));
          }
        }
        return;
      }
      case "BlockStatement":
      case "StaticBlock": {
        const s = new Set();
        blockDecls(n.body, s);
        n.body.forEach((c) => walk(c, [...scopes, s], n, "body"));
        return;
      }
      case "SwitchStatement": {
        walk(n.discriminant, scopes, n, "discriminant");
        const s = new Set();
        n.cases.forEach((c) => blockDecls(c.consequent, s));
        const inner = [...scopes, s];
        n.cases.forEach((c) => {
          if (c.test) walk(c.test, inner, c, "test");
          c.consequent.forEach((x) => walk(x, inner, c, "consequent"));
        });
        return;
      }
      case "ForStatement":
      case "ForInStatement":
      case "ForOfStatement": {
        const s = new Set();
        const init = n.init ?? n.left;
        if (init?.type === "VariableDeclaration" && init.kind !== "var") init.declarations.forEach((d) => patNames(d.id).forEach((x) => s.add(x)));
        const inner = [...scopes, s];
        for (const c of children(n)) {
          if (c.type === "VariableDeclaration") c.declarations.forEach((d) => { walkPattern(d.id, inner); if (d.init) walk(d.init, inner, d, "init"); });
          else walk(c, inner, n, null);
        }
        return;
      }
      case "CatchClause": {
        const s = new Set();
        if (n.param) patNames(n.param).forEach((x) => s.add(x));
        const inner = [...scopes, s];
        if (n.param) walkPattern(n.param, inner);
        walk(n.body, inner, n, "body");
        return;
      }
      case "VariableDeclarator":
        walkPattern(n.id, scopes);
        if (n.init) walk(n.init, scopes, n, "init");
        return;
      case "ThisExpression":
        if (!declared(scopes, "this")) free.add("this");
        return;
    }
    for (const k of Object.keys(n)) {
      if (k === "type" || k === "start" || k === "end") continue;
      const v = n[k];
      if (Array.isArray(v)) v.forEach((c) => c && typeof c.type === "string" && walk(c, scopes, n, k));
      else if (v && typeof v.type === "string") walk(v, scopes, n, k);
    }
  }
  function walkPattern(p, scopes) {
    if (!p) return;
    // pattern: identifiers are declarations (skip); defaults and computed keys are reads
    if (p.type === "AssignmentPattern") {
      walkPattern(p.left, scopes);
      walk(p.right, scopes, p, "right");
    } else if (p.type === "ObjectPattern") {
      for (const q of p.properties) {
        if (q.type === "RestElement") walkPattern(q.argument, scopes);
        else {
          if (q.computed) walk(q.key, scopes, q, "key");
          walkPattern(q.value, scopes);
        }
      }
    } else if (p.type === "ArrayPattern") p.elements.forEach((e) => walkPattern(e, scopes));
    else if (p.type === "RestElement") walkPattern(p.argument, scopes);
    else if (p.type === "MemberExpression") walk(p, scopes, null, null);
  }
  walk(node, [], null, null);
  return free;
}
