"djs-dump";
"use strict";
$djs.seed(2135587861, -1640531527, 2);
globalThis.$dump = {};
$dump.b0 = Object.getPrototypeOf(function* () {}).prototype;
$dump.b1 = Object.getPrototypeOf(function* () {}).prototype["next"];
$dump.s8 = {};
$dump.p1 = $djs.resume(1, "later", [1]);
$dump.n3 = {};
var tools = { __proto__: null, echo: undefined, fail: undefined };
$dump.n4 = $djs.host("done");
$dump.n5 = $djs.host("output");
var later = $djs.host("later");
$dump.n6 = new Map();
$dump.s2 = (() => {
const $s1 = (() => {
  let impl = (args) => roundTrip(args);
  return { $f4: impl, $f37: (0, (args) => {
      let json;
      try {
        json = roundTrip(args);
      } catch (error) {
        return Promise.reject(error);
      }
      return promiseThen.call(later(json), impl);
    }) };
})();
const $s2 = (() => {
  let impl = (args) => {
      throw new ErrorCtor(args && args.message !== undefined ? StringCtor(args.message) : "tool failed");
    };
  return { $f38: (0, (args) => {
      let json;
      try {
        json = roundTrip(args);
      } catch (error) {
        return Promise.reject(error);
      }
      return promiseThen.call(later(json), impl);
    }), $f55: impl };
})();
  function roundTrip(value) {
    const json = stringify(value);
    return json === undefined ? undefined : parse(json);
  }
  let EXIT = $dump.n3;
  function done(kind, text) {
    if (finished) return;
    finished = true;
    hostDone(kind, text);
  }
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
  let stringify = JSON.stringify;
  let parse = JSON.parse;
  let console;
  let promiseThen = Promise.prototype.then;
  let finished = false;
  function emit(text) {
    if (!finished) output(text);
  }
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
  let hostDone = $dump.n4;
  let ErrorCtor = Error;
  let StringCtor = String;
  function errorText(error) {
    const head = error.message ? error.name + ": " + error.message : StringCtor(error.name);
    const frames = typeof error.stack === "string" ? error.stack.split("\n").filter((line) => line.trim()) : [];
    return [head, ...frames].join("\n");
  }
  let output = $dump.n5;
  function outputText(value) {
    if (value === undefined || value === null || (typeof value !== "object" && typeof value !== "function")) {
      return StringCtor(value);
    }
    const json = stringify(value);
    return json === undefined ? StringCtor(value) : json;
  }
  let TypeErrorCtor = TypeError;
  let stored = $dump.n6;
  return { ...$s1, ...$s2, run(fn) {
      let promise;
      try {
        // Like pi: the script is `async (tools, console) => {...}`.
        promise = fn(tools, console);
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
    }, stalled() {
      if (finished) return false;
      done(
        "throw",
        "Error: The script is waiting on a promise that can never settle: no tool call is pending, and timers do not exist here.",
      );
      return true;
    }, $f7: (0, (...args) => emit(args.map(format).join(" "))), $f8: (0, (...args) => emit(args.map(format).join(" "))), $f9: (0, (...args) => emit(args.map(format).join(" "))), $f10: (0, (...args) => emit(args.map(format).join(" "))), $f11: (0, (...args) => emit(args.map(format).join(" "))), roundTrip, done, describe, emit, format, $f41: (0, (value) => {
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
        }), $f42: (0, (error) => {
          if (error !== EXIT) done("throw", describe(error));
        }), errorText, text: function text(value) {
    let rendered;
    try {
      rendered = outputText(value);
    } catch (error) {
      throw new TypeErrorCtor(error instanceof ErrorCtor ? error.message : StringCtor(error));
    }
    emit(rendered);
  }, outputText, exit: function exit() {
    done("completed", "");
    throw EXIT;
  }, store: function store(key, value) {
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
  }, load: function load(key) {
    if (typeof key !== "string") throw new TypeErrorCtor("load() key must be a string");
    const json = stored.get(key);
    return json === undefined ? undefined : parse(json);
  }, $set(v) { ({ console } = v); } };
})();
var console = { log: $dump.s2.$f7, info: $dump.s2.$f8, warn: $dump.s2.$f9, error: $dump.s2.$f10, debug: $dump.s2.$f11 };
var text = $dump.s2.text;
var exit = $dump.s2.exit;
var ALL_TOOLS = [{ name: "echo", description: "Returns its arguments." }, { name: "fail", description: "Rejects with an Error carrying args.message." }];
var store = $dump.s2.store;
var load = $dump.s2.load;
$dump.p7 = Promise.prototype.then.call($dump.p1, $dump.s2.$f4, undefined);
$dump.n10 = { __proto__: $dump.b0 };
$dump.ga11 = () => {
$djs.k($dump.s8.$Ka9, { f: $dump.s8.g, at: "yield#1", segs: [66,35,5,74,42,3,128,47,12], scaf: [["for",19,38],["site",47,114]] }, $dump.n10);
$dump.a9 = $dump.s8.$Ka9.call(undefined, 2);
$dump.a9.next();
};
$dump.Fa11 = $djs.source("codemode/plain-fuzz/generator-capturing-for-of.js", "6d50a03dd0a4eced", "", 1);
$dump.a11 = $djs.k(async (tools) => {
const $djs_L = $djs.L;
let n = 1;
Object.assign($dump.s8, { g: function* g(k) {
  for (let i = 0; i < k; i++) yield i + n;
}, $Ka9: function* (k) {
{ let $djs_f1 = true; for (let i = 0; $djs_f1 || (i < k); i++) { if ($djs_f1) { $djs_f1 = false; 
yield; } else yield i + n; } }
} });
var g = $dump.s8.g;
$dump.ga11();
{ let $djs_f1 = true; for (const $djs_v1 of $djs_L($dump.a9, $dump.b1)) { if ($djs_f1) { $djs_f1 = false; {
const x = 1;
(n = 1 + await $dump.p7);
} } else { const x = $djs_v1; {
  n += await tools.echo(x);
} } } }
return n;
}, { f: $dump.Fa11, segs: [458,268,1,494,247,7,513,264,31,551,296,9], scaf: [["for-of",242,358],["site",273,466]], cmap: [["assign.old@268",1,1]] })(tools);
$dump.p12 = Promise.prototype.then.call($dump.a11, $dump.s2.$f41, $dump.s2.$f42);
tools.echo = $dump.s2.$f37;
tools.fail = $dump.s2.$f38;
Object.defineProperty($dump.s2.$f4, "name", { value: "echo" });
Object.defineProperty($dump.s2.$f55, "name", { value: "fail" });
$dump.s2.$set({ console: console });
Object.freeze($dump.n3);
Object.freeze(tools);
Object.freeze(console);
Object.freeze(ALL_TOOLS[0]);
Object.freeze(ALL_TOOLS[1]);
Object.freeze(ALL_TOOLS);
try { ({ run: $dump.s2.run, stalled: $dump.s2.stalled }); } finally { delete globalThis.$dump; delete globalThis.$djs; }
