// Test driver for running test262 with dquickts' qjs (`qjs -m -e <this> <spec>`).
//
// <spec> is JSON: {"harness": [[name, path], ...], "test": path, "strict": bool}.
// The harness files and the test are each evaluated as a Script in the global scope
// (std.evalScript). The test is compiled first and run second, so parse errors are
// told apart from runtime errors. The outcome is written as one line:
//
//   \x01T262 <status> <file> <name> <message>
//
// where status is completed | harness | parse | runtime and the other fields are
// JSON-quoted strings. Everything else on stdout comes from the test's print().
//
// Tests may tamper with builtins, so everything used after the test ran is captured
// up front and nothing relies on prototype methods or the iterator protocol.
import * as std from "std";

const out = std.out;
const evalScript = std.evalScript;
const loadFile = std.loadFile;
const toStr = String;
const apply = Reflect.apply;
const charCodeAt = String.prototype.charCodeAt;
const defineProperty = Object.defineProperty;
const transfer = ArrayBuffer.prototype.transfer;
const isDetached = Object.getOwnPropertyDescriptor(ArrayBuffer.prototype, "detached").get;
const HEX = "0123456789abcdef";

const spec = JSON.parse(scriptArgs[scriptArgs.length - 1]);
const prefix = spec.strict ? '"use strict";' : "";

function quote(s) {
  let r = '"';
  for (let i = 0; i < s.length; i++) {
    const c = apply(charCodeAt, s, [i]);
    if (c < 0x20 || c === 0x22 || c === 0x5c || c > 0x7e) {
      r += "\\u" + HEX[(c >> 12) & 15] + HEX[(c >> 8) & 15] + HEX[(c >> 4) & 15] + HEX[c & 15];
    } else {
      r += s[i];
    }
  }
  return r + '"';
}

function report(status, file, e, thrown) {
  let name = "";
  let message = "";
  if (thrown) {
    try {
      if (e !== null && (typeof e === "object" || typeof e === "function")) {
        const c = e.constructor;
        name = typeof c === "function" && typeof c.name === "string" && c.name !== "" ? c.name : toStr(e.name);
        message = toStr(e.message);
      } else {
        name = typeof e;
        message = toStr(e);
      }
    } catch (_) {
      message = "<exception while describing the thrown value>";
    }
  }
  out.puts("\x01T262 " + status + " " + quote(file) + " " + quote(name) + " " + quote(message) + "\n");
  out.flush();
}

function load(path) {
  const src = loadFile(path);
  if (src === null) throw new Error("cannot read " + path);
  return src;
}

const $262 = {
  global: globalThis,
  evalScript(src) {
    return evalScript(prefix + toStr(src));
  },
  // DetachArrayBuffer is a no-op on a detached buffer; transfer() would throw.
  detachArrayBuffer(buffer) {
    if (!apply(isDetached, buffer, [])) apply(transfer, buffer, []);
  },
  gc() {},
};
defineProperty(globalThis, "$262", { value: $262, writable: true, configurable: true, enumerable: false });
defineProperty(globalThis, "print", { enumerable: false });

function main() {
  for (let i = 0; i < spec.harness.length; i++) {
    const name = spec.harness[i][0];
    try {
      evalScript(prefix + load(spec.harness[i][1]));
    } catch (e) {
      report("harness", name, e, true);
      return;
    }
  }
  let compiled;
  try {
    compiled = evalScript(prefix + load(spec.test), { compile_only: true });
  } catch (e) {
    report("parse", "", e, true);
    return;
  }
  try {
    evalScript(compiled, { eval_function: true });
  } catch (e) {
    report("runtime", "", e, true);
    return;
  }
  report("completed", "", undefined, false);
}

main();
