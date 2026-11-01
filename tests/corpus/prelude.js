// The difftest codemode prelude, evaluated as a strict Script by every engine before a
// case. It mirrors what pi's codemode shows the model (pi-private
// packages/codemode/src/runtime/prelude-source.ts): `console.*` and `text()` output, the
// return value, and the uncaught error, all formatted by the functions copied below
// (`errorText`, `format`, `outputText`, `describe`). See TESTING.md ("Differential tests")
// and docs/decisions.md (D1100).
//
// Left out on purpose: pi's lockdown (freezing every built-in) and the Proxy that guards
// `tools`; `image()`; store size limits. The fake `tools` resolve through promise jobs only,
// so a script that awaits anything else can never settle (reported like pi does).
//
// The prelude evaluates to `(host) => api`. The engine glue passes
//   host.output(text)        one output item (console.* or text())
//   host.done(kind, text)    kind: "completed" (no value), "return" (the value as the
//                            model sees it) or "throw" (the error text); called once
// and calls api.run(fn) with the compiled case (an async function), and api.stalled()
// once nothing is left to run.
(function (host) {
  "use strict";

  // Captured before the case runs: cases may replace built-ins.
  const stringify = JSON.stringify;
  const parse = JSON.parse;
  const promiseThen = Promise.prototype.then;
  const promiseResolve = Promise.resolve.bind(Promise);
  const ErrorCtor = Error;
  const TypeErrorCtor = TypeError;
  const StringCtor = String;
  const output = host.output;
  const hostDone = host.done;
  // Thrown by exit() to unwind the script after it already reported success.
  const EXIT = Object.freeze({});
  let finished = false;

  function done(kind, text) {
    if (finished) return;
    finished = true;
    hostDone(kind, text);
  }

  // From pi: "Name: message" like V8, then the stack frames. (The difftest runner drops
  // frame lines before comparing; stack formats are a separate topic.)
  function errorText(error) {
    const head = error.message ? error.name + ": " + error.message : StringCtor(error.name);
    const frames =
      typeof error.stack === "string" ? error.stack.split("\n").filter((line) => line.trim()) : [];
    return [head, ...frames].join("\n");
  }

  // From pi: console.* arguments.
  function format(value) {
    if (typeof value === "string") return value;
    if (value instanceof ErrorCtor) return errorText(value);
    try {
      const json = stringify(value);
      return json === undefined ? StringCtor(value) : json;
    } catch {
      return StringCtor(value);
    }
  }

  // From pi: text() arguments. Primitives become their string form, everything else JSON.
  function outputText(value) {
    if (value === undefined || value === null || (typeof value !== "object" && typeof value !== "function")) {
      return StringCtor(value);
    }
    const json = stringify(value);
    return json === undefined ? StringCtor(value) : json;
  }

  // From pi's describeError plus the host's formatError: what the model reads for an
  // uncaught error.
  function describe(error) {
    try {
      if (error instanceof ErrorCtor) {
        StringCtor(error.name);
        StringCtor(error.message);
        return errorText(error);
      }
      return "Error: " + StringCtor(format(error));
    } catch {
      return "Error: The script threw a value that cannot be described";
    }
  }

  function emit(text) {
    if (!finished) output(text);
  }

  const console = {};
  for (const level of ["log", "info", "warn", "error", "debug"]) {
    console[level] = (...args) => emit(args.map(format).join(" "));
  }
  Object.freeze(console);

  function text(value) {
    let rendered;
    try {
      rendered = outputText(value);
    } catch (error) {
      throw new TypeErrorCtor(error instanceof ErrorCtor ? error.message : StringCtor(error));
    }
    emit(rendered);
  }

  function exit() {
    done("completed", "");
    throw EXIT;
  }

  // Fake tools: arguments and results make a JSON round trip, like pi's.
  function roundTrip(value) {
    const json = stringify(value);
    return json === undefined ? undefined : parse(json);
  }
  const toolList = {
    echo: (args) => roundTrip(args),
    fail: (args) => {
      throw new ErrorCtor(args && args.message !== undefined ? StringCtor(args.message) : "tool failed");
    },
  };
  const tools = Object.create(null);
  for (const name of Object.keys(toolList)) {
    const impl = toolList[name];
    tools[name] = (args) => {
      let json;
      try {
        json = roundTrip(args);
      } catch (error) {
        return Promise.reject(error);
      }
      return promiseThen.call(promiseResolve(json), impl);
    };
  }
  Object.freeze(tools);
  const ALL_TOOLS = Object.freeze([
    Object.freeze({ name: "echo", description: "Returns its arguments." }),
    Object.freeze({ name: "fail", description: "Rejects with an Error carrying args.message." }),
  ]);

  // store/load: JSON values, copied on the way in and out.
  const stored = new Map();
  function store(key, value) {
    if (typeof key !== "string") throw new TypeErrorCtor("store() key must be a string");
    if (value === undefined) {
      stored.delete(key);
      return;
    }
    let json;
    try {
      json = stringify(value);
    } catch (error) {
      throw new TypeErrorCtor("store(" + stringify(key) + ") value is not JSON-serializable: " + format(error));
    }
    if (json === undefined) throw new TypeErrorCtor("store(" + stringify(key) + ") value is not JSON-serializable");
    stored.set(key, json);
  }
  function load(key) {
    if (typeof key !== "string") throw new TypeErrorCtor("load() key must be a string");
    const json = stored.get(key);
    return json === undefined ? undefined : parse(json);
  }

  for (const [name, value] of [
    ["console", console],
    ["text", text],
    ["exit", exit],
    ["tools", tools],
    ["ALL_TOOLS", ALL_TOOLS],
    ["store", store],
    ["load", load],
  ]) {
    Object.defineProperty(globalThis, name, { value, writable: true, enumerable: true, configurable: true });
  }

  return {
    run(fn) {
      let promise;
      try {
        promise = fn();
      } catch (error) {
        if (error !== EXIT) done("throw", describe(error));
        return;
      }
      promiseThen.call(
        promise,
        (value) => {
          // pi serializes the value in the VM and the host shows strings as they are and
          // everything else as compact JSON.
          let json;
          try {
            json = stringify(value);
          } catch (error) {
            done("throw", describe(error));
            return;
          }
          if (json === undefined) {
            done("completed", "");
            return;
          }
          const parsed = parse(json);
          done("return", typeof parsed === "string" ? parsed : stringify(parsed));
        },
        (error) => {
          if (error !== EXIT) done("throw", describe(error));
        },
      );
    },
    stalled() {
      if (finished) return false;
      done(
        "throw",
        "Error: The script is waiting on a promise that can never settle: no tool call is pending, and timers do not exist here.",
      );
      return true;
    },
  };
});
