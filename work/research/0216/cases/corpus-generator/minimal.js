"djs-dump";
"use strict";
$djs.seed(2135587861,-1640531527);
globalThis.$d={};
$d.n3={};
var tools = { __proto__: null, echo: undefined, fail: undefined };
var later = $djs.host("later");
$d.n6=new Map();
$d.s2 = (() => {
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
  let EXIT = $d.n3;
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
  let hostDone = $djs.host("done");
  let ErrorCtor = Error;
  let StringCtor = String;
  function errorText(error) {
    const head = error.message ? error.name + ": " + error.message : StringCtor(error.name);
    const frames = typeof error.stack === "string" ? error.stack.split("\n").filter((line) => line.trim()) : [];
    return [head, ...frames].join("\n");
  }
  let output = $djs.host("output");
  function outputText(value) {
    if (value === undefined || value === null || (typeof value !== "object" && typeof value !== "function")) {
      return StringCtor(value);
    }
    const json = stringify(value);
    return json === undefined ? StringCtor(value) : json;
  }
  let TypeErrorCtor = TypeError;
  let stored = $d.n6;
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
var console = { log: $d.s2.$f7, info: $d.s2.$f8, warn: $d.s2.$f9, error: $d.s2.$f10, debug: $d.s2.$f11 };
var text = $d.s2.text;
var exit = $d.s2.exit;
var ALL_TOOLS = [{ name: "echo", description: "Returns its arguments." }, { name: "fail", description: "Rejects with an Error carrying args.message." }];
var store = $d.s2.store;
var load = $d.s2.load;
$djs.k(async (tools) => {
let n = 1;
function* g(k) {
  for (let i = 0; i < k; i++) yield i + n;
}
const $g = $djs.k(function* (k) {
let i = 0;
yield;
for (i++; i < k; i++) yield i + n;
},g,"yield#1")(2);
$g.next();
{ let $f1 = true; for (const $v1 of $djs.L($g)) { if ($f1) { $f1 = false; {
n += await $djs.resume(1,"later",[1]).then($d.s2.$f4);
} } else { const x = $v1; {
  n += await tools.echo(x);
} } } }
return n;
},$djs.source("codemode/plain-fuzz/generator-capturing-for-of.js","6d50a03dd0a4eced","",1))(tools).then($d.s2.$f41,$d.s2.$f42);
tools.echo = $d.s2.$f37;
tools.fail = $d.s2.$f38;
Object.defineProperty($d.s2.$f4, "name", { value: "echo" });
Object.defineProperty($d.s2.$f55, "name", { value: "fail" });
$d.s2.$set({console:console});
Object.freeze($d.n3);
Object.freeze(tools);
Object.freeze(console);
Object.freeze(ALL_TOOLS[0]);
Object.freeze(ALL_TOOLS[1]);
Object.freeze(ALL_TOOLS);
({ run: $d.s2.run, stalled: $d.s2.stalled })
