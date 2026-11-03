// Case driver for difftest with dquickts' qjs (`qjs -m -e <this> <spec>`).
//
// <spec> is JSON: {"prelude": path, "case": path, "ts": bool, "legacy": bool,
// "metadata": bool} (the last two: TypeScript legacy decorators and their metadata).
// The prelude is evaluated as a strict Script; the case becomes the body of a strict
// `async (tools, console) =>` arrow function (as in pi, D008) and is
// run through the prelude's api.run(). Results are written as single lines:
//
//   \x01OUT <json string>             one output item
//   \x01DONE <kind> <json string>     the result (kind: completed | return | throw)
//
// Anything else on stdout is unexpected (the case has no other way to print).
import * as std from "std";
import * as os from "os";

const out = std.out;
const evalScript = std.evalScript;
const loadFile = std.loadFile;
const stringify = JSON.stringify;
const spec = JSON.parse(scriptArgs[scriptArgs.length - 1]);

function line(text) {
  out.puts(text + "\n");
  out.flush();
}

function read(path) {
  const src = loadFile(path);
  if (src === null) throw new Error("cannot read " + path);
  return src;
}

// qjs host globals that djs does not have; cases must not depend on them.
for (const name of ["print", "scriptArgs", "__loadScript", "execArgv", "argv0", "gc", "navigator"]) {
  delete globalThis[name];
}

let done = false;
const promiseResolve = Promise.resolve.bind(Promise);
const promiseReject = Promise.reject.bind(Promise);
const host = {
  // djs settles this at the next idle point (an async host call, 0173, D2590); here the
  // job queue is all there is.
  later(value, rejected) {
    return rejected ? promiseReject(value) : promiseResolve(value);
  },
  output(text) {
    line("\x01OUT " + stringify(text));
  },
  done(kind, text) {
    done = true;
    line("\x01DONE " + kind + " " + stringify(text));
  },
};

// loadScript, not evalScript: the prelude's frames carry its path, so that a
// `difftest: stacks` case can tell them from its own (`<evalScript>`).
const setup = std.loadScript(spec.prelude);
const api = setup(host);
let fn;
try {
  // Same line as the case's first line, so line numbers match the case file.
  fn = evalScript('"use strict";(async (tools, console) => {' + read(spec.case) + "\n})", {
    typescript: spec.ts,
    experimentalDecorators: spec.legacy,
    emitDecoratorMetadata: spec.metadata,
  });
} catch (e) {
  api.run(() => {
    throw e;
  });
}
if (fn !== undefined) api.run(fn);
// Timers run only once the job queue is empty: if the case has not settled by then,
// nothing can settle it any more.
os.setTimeout(() => {
  if (!done) api.stalled();
}, 0);
