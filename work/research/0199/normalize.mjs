// normalize.mjs: origin tracking. A format 2 dump of a run that resumed a plain dump holds
// activations of continuations K. Each K carries `$djs = {f, at, segs, scaf}`; this maps
// every such activation back to an activation of the original function f at the original
// site: K's site through the segments K copied verbatim from f, K's slots to f's slot
// names (temporaries renamed by offset, the for-of scaffold's iterator record unwrapped,
// K's own `$djs_*` locals dropped). The result is a format 2 dump of original functions
// only, which toPlain turns into a plain dump generated from the original text again.
import { parseScript, parseFunctionText, sites, children, isFn, refuse } from "./plain.mjs";

const memberChain = (n) => {
  const out = [];
  while (n.type === "MemberExpression" && !n.computed) {
    out.unshift(n.property.name);
    n = n.object;
  }
  return n.type === "Identifier" && n.name === "$dump" ? out : null;
};

export function normalize(text) {
  const ast = parseScript(text);
  const src = (n) => text.slice(n.start, n.end);
  const edits = []; // [start, end, text]
  const defs = new Map(); // $dump name -> statement
  const factories = new Map();
  for (const st of ast.body) {
    const e = st.expression;
    if (e?.type === "AssignmentExpression" && memberChain(e.left)?.length === 1) {
      defs.set(memberChain(e.left)[0], st);
      const r = e.right;
      if (r.type === "CallExpression" && r.callee.type === "ArrowFunctionExpression" && r.arguments.length === 0) factories.set(memberChain(e.left)[0], r);
    }
  }
  // A factory's object (and its sub-factories', spread in): key -> Property; var -> init.
  function factoryParts(call, out = { props: new Map(), vars: new Map() }) {
    for (const st of call.callee.body.body) {
      if (st.type === "ReturnStatement")
        for (const p of st.argument.properties) {
          if (p.type === "Property") out.props.set(p.key.type === "Identifier" ? p.key.name : p.key.value, p);
        }
      else if (st.type === "VariableDeclaration")
        for (const d of st.declarations) {
          if (d.init?.type === "CallExpression" && d.init.callee.type === "ArrowFunctionExpression") factoryParts(d.init, out);
          else out.vars.set(d.id.name, d.init);
        }
    }
    return out;
  }
  // The function a reference names: {text, node}.
  function fnOf(n) {
    if (isFn(n)) return { text: src(n), node: n };
    const ch = memberChain(n);
    if (ch?.length === 2 && factories.has(ch[0])) {
      const p = factoryParts(factories.get(ch[0])).props.get(ch[1]);
      if (p) return { text: p.method || p.kind !== "init" ? src(p) : fnText(p.value), node: p.value };
    }
    if (ch?.length === 1 && defs.has(ch[0])) return { text: fnText(defs.get(ch[0]).expression.right), node: defs.get(ch[0]).expression.right };
    if (n.type === "Identifier") {
      for (const st of ast.body) {
        if (st.type === "FunctionDeclaration" && st.id.name === n.name) return { text: src(st), node: st };
        if (st.type === "VariableDeclaration") for (const d of st.declarations) if (d.id.name === n.name && d.init) return { text: fnText(d.init), node: d.init };
      }
    }
    return null;
  }
  function fnText(n) {
    if (n.type === "CallExpression" && src(n.callee) === "$dump.fn") return src(n.arguments[0].properties[0]);
    return src(n);
  }
  const objProps = (o) => {
    const m = new Map();
    for (const p of o.properties) m.set(p.key.type === "Identifier" ? p.key.name : p.key.value, p);
    return m;
  };
  // Origin records: `Object.defineProperty(K, "$djs", { value: V, ... })`.
  const origins = new Map(); // K reference text -> {f, at, segs, scaf}
  for (const st of ast.body) {
    const e = st.expression;
    if (e?.type !== "CallExpression" || src(e.callee) !== "Object.defineProperty" || e.arguments[1]?.value !== "$djs") continue;
    let v = objProps(e.arguments[2]).get("value").value;
    let rec;
    const patches = new Map();
    if (v.type === "ObjectExpression") rec = objProps(v);
    else {
      const name = memberChain(v)[0];
      rec = objProps(defs.get(name).expression.right);
      // `$dump.nK.f = ...` patches (a value the record could not hold inline)
      for (const s2 of ast.body)
        if (s2.expression?.type === "AssignmentExpression" && memberChain(s2.expression.left)?.length === 2 && memberChain(s2.expression.left)[0] === name)
          patches.set(memberChain(s2.expression.left)[1], s2.expression.right);
    }
    const val = (k) => patches.get(k) ?? rec.get(k)?.value;
    const fnode = val("f");
    origins.set(src(e.arguments[0]), {
      f: fnode,
      at: val("at").value,
      segs: val("segs").elements.map((x) => (x.type === "UnaryExpression" ? -x.argument.value : x.value)),
      scaf: val("scaf").elements.map((x) => x.elements.map((y) => (y.type === "Literal" ? y.value : -y.argument.value))),
      temps: (val("temps")?.elements ?? []).map((x) => x.elements.map((y) => y.value)),
    });
    edits.push([st.start, st.end, ""]);
  }
  let mapped = 0;
  for (const st of ast.body) {
    const e = st.expression;
    if (e?.type !== "AssignmentExpression" || e.right.type !== "CallExpression" || src(e.right.callee) !== "$dump.resume") continue;
    const [fnNode, optsNode] = e.right.arguments;
    const o = origins.get(src(fnNode));
    if (!o) continue;
    mapped++;
    const k = fnOf(fnNode) ?? refuse("continuation without text");
    const kp = parseFunctionText(k.text);
    const opts = objProps(optsNode);
    const [kind, nth] = opts.get("at").value.value.split("#");
    const ksite = sites(kp.fn)[kind]?.[Number(nth) - 1] ?? refuse(`no site ${kind}#${nth} in K`);
    // f and its text
    let ftext;
    let fref;
    if (o.f.type === "Literal" && typeof o.f.value === "string") {
      ftext = o.f.value;
      fref = ftext; // an inline function again
    } else {
      const ff = fnOf(o.f) ?? refuse("origin function not found");
      ftext = ff.text;
      fref = src(o.f);
    }
    const fp = parseFunctionText(ftext);
    const toF = (koff) => {
      for (let i = 0; i < o.segs.length; i += 3) if (o.segs[i] <= koff && koff < o.segs[i] + o.segs[i + 2]) return o.segs[i + 1] + (koff - o.segs[i]);
      for (const s of o.scaf) if (s[2] === koff) return s[1];
      return null;
    };
    const foff = toF(ksite.start) ?? refuse("K's site is not in f's text");
    const fs = sites(fp.fn)[kind];
    const fi = fs.findIndex((x) => x.start === foff);
    if (fi < 0) refuse(`no ${kind} at ${foff} in f`);
    const at = `${kind}#${fi + 1}`;
    // K still at the original site: f's temporaries are K's consts (or literals)
    const atSite = o.scaf.some((s) => s[0] === "site" && s[2] === ksite.start);
    // slots
    const kslots = objProps(opts.get("slots").value);
    const out = [];
    for (const [name, p] of kslots) {
      if (name.startsWith("$djs_")) continue;
      const m = /^(.*)@(\d+)$/.exec(name);
      if (!m) {
        out.push([name, src(p.value)]);
        continue;
      }
      const [, construct, off] = m;
      const scaf = o.scaf.find((s) => s[2] === Number(off));
      if (scaf && (construct === "for-of.iterator" || construct === "for-await.iterator")) {
        // the scaffold's iterable: unwrap the saved iterator record
        const w = unwrapLoop(p.value);
        const c = construct.split(".")[0];
        out.push([`${c}.iterator@${scaf[1]}`, w.it]);
        out.push([`${c}.next@${scaf[1]}`, w.next]);
        continue;
      }
      if (scaf && /\.next$/.test(construct)) continue; // the wrapper's next
      const f = toF(Number(off));
      if (f === null) {
        if (atSite) continue; // K's own rewriting of the site's statement
        refuse("re-suspended inside the rewritten statement");
      }
      out.push([`${construct}@${f}`, src(p.value)]);
    }
    if (atSite)
      for (const [fname, isConst, v] of o.temps) {
        if (!isConst) out.push([fname, v]);
        else if (kslots.has(v)) out.push([fname, src(kslots.get(v).value)]);
        else refuse(`K's temporary ${v} is gone`);
      }
    // a suspended finally: its completion is K's constant and local
    for (const s of o.scaf)
      if (s[0] === "finally" && kslots.has(s[4])) {
        out.push([`completion.kind@${s[1]}`, String(s[3])]);
        out.push([`completion.value@${s[1]}`, src(kslots.get(s[4]).value)]);
      }
    const keep = ["awaiting", "queue"].filter((x) => opts.has(x)).map((x) => `${x}: ${src(opts.get(x).value)}`);
    if (opts.has("this") && fp.kind !== "arrow") keep.push(`this: ${src(opts.get("this").value)}`);
    const fexpr = typeof fref === "string" && o.f.type === "Literal" ? ftext : fref;
    const slotsText = out.map(([k2, v]) => `${JSON.stringify(k2)}: ${v}`).join(", ");
    edits.push([e.right.start, e.right.end, `$dump.resume(${fexpr}, { at: ${JSON.stringify(at)}, ${keep.join(", ")}${keep.length ? ", " : ""}slots: { ${slotsText} } })`]);
  }
  function unwrapLoop(n) {
    // the wrapper object of `$dump.L`: its `next` closes over `it` and `next`
    const ch = memberChain(n);
    const obj = ch?.length === 1 ? defs.get(ch[0])?.expression.right : n.type === "ObjectExpression" ? n : null;
    if (!obj || obj.type !== "ObjectExpression") refuse("loop wrapper not found");
    const nextProp = objProps(obj).get("next").value;
    const fch = memberChain(nextProp);
    if (!fch || fch.length !== 2 || !factories.has(fch[0])) refuse("loop wrapper's next not found");
    const vars = factoryParts(factories.get(fch[0])).vars;
    if (!vars.has("it") || !vars.has("next")) refuse("loop wrapper's record not found");
    return { it: src(vars.get("it")), next: src(vars.get("next")) };
  }
  let outText = text;
  edits.sort((a, b) => b[0] - a[0]);
  for (const [s, e, t] of edits) outText = outText.slice(0, s) + t + outText.slice(e);
  return { text: dce(outText), mapped };
}

// Drops what nothing live references any more: old continuations, their helpers and
// scaffolding. Roots: activations, open calls, the handle, the script's own globals.
export function dce(text) {
  const ast = parseScript(text);
  const src = (n) => text.slice(n.start, n.end);
  // $dump references of live code: not inside functions (a continuation's text names the
  // previous dump's values), except factories' bodies, which run.
  const refs = (n, out = new Set(), parent = null) => {
    if (isFn(n) && !(parent?.type === "CallExpression" && parent.callee === n && n.type === "ArrowFunctionExpression")) return out;
    const ch = n.type === "MemberExpression" ? memberChain(n) : null;
    if (ch && ch.length) {
      out.add(ch[0]);
      if (ch.length >= 2) out.add(ch[0] + "." + ch[1]);
      return out;
    }
    for (const c of children(n)) refs(c, out, n);
    return out;
  };
  const stmts = ast.body.map((st) => {
    const e = st.expression;
    let defines = null;
    let target = null;
    if (e?.type === "AssignmentExpression" && memberChain(e.left)?.length === 1) defines = memberChain(e.left)[0];
    else if (e?.type === "AssignmentExpression" && memberChain(e.left)?.length >= 2) target = memberChain(e.left)[0];
    else if (e?.type === "CallExpression" && src(e.callee) === "Object.defineProperty" && e.arguments[0] && memberChain(e.arguments[0])) target = memberChain(e.arguments[0])[0];
    else if (e?.type === "CallExpression" && e.callee.type === "MemberExpression" && memberChain(e.callee)?.length >= 2 && /^(\$set|next)$/.test(e.callee.property.name ?? "")) target = memberChain(e.callee)[0];
    else if (e?.type === "CallExpression" && src(e.callee) === "$dump.keep") target = memberChain(e.arguments[0])?.[0] ?? null;
    const isRoot =
      !defines && !target
        ? true
        : defines && e.right.type === "CallExpression" && /^\$dump\.(resume|pending)$/.test(src(e.right.callee));
    return { st, defines, target, refs: refs(st), root: isRoot };
  });
  const live = new Set();
  const liveStmt = new Set();
  let changed = true;
  while (changed) {
    changed = false;
    for (const s of stmts) {
      if (liveStmt.has(s)) continue;
      const on = s.root || (s.defines && live.has(s.defines)) || (s.target && live.has(s.target));
      if (!on) continue;
      liveStmt.add(s);
      for (const r of s.refs) if (!r.includes(".")) live.add(r);
      changed = true;
    }
  }
  // Factory props that are old continuations nobody calls ($K...).
  const liveKeys = new Set();
  for (const s of liveStmt) for (const r of s.refs) if (r.includes(".")) liveKeys.add(r);
  const edits = [];
  for (const s of stmts) {
    if (!liveStmt.has(s)) {
      edits.push([s.st.start, s.st.end + (text[s.st.end] === "\n" ? 1 : 0), ""]);
      continue;
    }
    if (s.defines && s.st.expression.right.type === "CallExpression" && s.st.expression.right.callee.type === "ArrowFunctionExpression") {
      (function visit(call) {
        for (const st of call.callee.body.body) {
          if (st.type === "ReturnStatement")
            for (const p of st.argument.properties)
              if (p.type === "Property" && p.key.type === "Identifier" && p.key.name.startsWith("$K") && !liveKeys.has(`${s.defines}.${p.key.name}`)) {
                let end = p.end;
                while (/[\s,]/.test(text[end]) && text[end] !== "}") end++;
                edits.push([p.start, end, ""]);
              }
          if (st.type === "VariableDeclaration") for (const d of st.declarations) if (d.init?.type === "CallExpression" && d.init.callee.type === "ArrowFunctionExpression") visit(d.init);
        }
      })(s.st.expression.right);
    }
  }
  edits.sort((a, b) => b[0] - a[0]);
  let out = text;
  for (const [a, b, t] of edits) out = out.slice(0, a) + t + out.slice(b);
  return out;
}
