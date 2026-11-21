// Builtin calls made inside the interpreter's loop, and a primitive's builtin method read
// from the cache: receivers of several kinds at one site, builtins replaced, shadowed and
// deleted while the caches are warm, calls that throw or convert an object.
function show(label, v) {
  console.log(label, typeof v === "string" ? JSON.stringify(v) : String(v));
}
function run(f) {
  try {
    return f();
  } catch (e) {
    return e.constructor.name + ": " + e.message;
  }
}

// One site, many receivers.
function str(x) {
  return x.toString();
}
const recv = ["ab", 12.5, { toString() { return "obj"; } }, true, Symbol("sy"), 10n, [1, 2], "cd", 7];
for (let round = 0; round < 3; round++) {
  const out = [];
  for (const x of recv) out.push(str(x));
  show("toString", out.join("|"));
}

// Hot calls on strings and numbers.
function work(s, n) {
  let acc = 0;
  for (let i = 0; i < 40; i++) {
    acc += s.trim().length + s.charCodeAt(i % s.length) + s.slice(1, 3).length + s.padStart(8, "-").length;
    acc += s.indexOf("b") + (n + i).toFixed(1).length + (n + i).toString().length + String(i).length;
    acc += Math.floor(i / 3) + Math.max(i, 5) + (s.startsWith(" a") ? 1 : 0) + s.hasOwnProperty("length");
  }
  return acc;
}
show("work", work(" abc ", 1.25));

// Replacing, shadowing and deleting builtins while the caches are warm.
const trim = String.prototype.trim;
String.prototype.trim = function () {
  return "T" + trim.call(this);
};
show("replaced trim", work(" abc ", 1.25));
String.prototype.trim = trim;
show("restored trim", work(" abc ", 1.25));
String.prototype.hasOwnProperty = function () {
  return 100;
};
show("shadowed hasOwnProperty", work(" abc ", 1.25));
delete String.prototype.hasOwnProperty;
show("unshadowed", work(" abc ", 1.25));
Math.unrelated = 1;
show("unrelated write", work(" abc ", 1.25));
const fixed = Number.prototype.toFixed;
delete Number.prototype.toFixed;
show("deleted toFixed", run(() => work(" abc ", 1.25)));
Number.prototype.toFixed = fixed;
const charCodeAt = String.prototype.charCodeAt;
Object.defineProperty(String.prototype, "charCodeAt", {
  get() {
    return () => 1000;
  },
  configurable: true,
});
show("getter charCodeAt", work(" abc ", 1.25));
delete String.prototype.charCodeAt;
show("no charCodeAt", run(() => work(" abc ", 1.25)));
String.prototype.charCodeAt = charCodeAt;
show("charCodeAt again", work(" abc ", 1.25));

// Calls that throw, and arguments that are converted.
function edge(s, n, arg) {
  const out = [];
  for (let i = 0; i < 4; i++) {
    out.push(run(() => n.toFixed(arg)));
    out.push(run(() => s.repeat(arg)));
    out.push(run(() => s.slice(arg)));
    out.push(run(() => s.padEnd(arg, arg)));
    out.push(run(() => Math.pow(arg, 2)));
  }
  return out.join("|");
}
let seen = "";
const conv = {
  valueOf() {
    seen += "v";
    return 2;
  },
  toString() {
    seen += "s";
    return "x";
  },
};
show("edge 1", edge("ab", 1.5, 1));
show("edge -1", edge("ab", 1.5, -1));
show("edge 200", edge("ab", 1.5, 200));
show("edge object", edge("ab", 1.5, conv));
show("conversions", seen);
show("edge symbol", edge("ab", 1.5, Symbol("q")));
show("edge undefined", edge("ab", 1.5, undefined));

// Deep frames, on the way down and back.
function deep(n, s) {
  const c = s.trim().length + Math.pow(n, 2) + s.charCodeAt(0);
  return n === 0 ? c : c + deep(n - 1, s) + Math.floor(n / 2);
}
show("deep", deep(300, " z "));

// Methods as values, and calls with other receivers.
const at = "abc".at, fix = (1.5).toFixed;
show("borrowed at", run(() => at.call("xyz", 1)));
show("borrowed toFixed", run(() => fix.call("xyz", 1)));
show("null receiver", run(() => at.call(null, 1)));
show("same function", "q".trim === String.prototype.trim && (5).toString === Number.prototype.toString);
