// mode: body
// `import()` compiles and rejects at run time: djs has no module loader (0058). The
// load error's message differs from the qjs CLI's (which has a loader), so only its
// name is printed.
const log = [];
const show = (e) => (e instanceof ReferenceError ? "ReferenceError" : String(e));
const p = import("nonexistent");
console.log(p instanceof Promise);
p.catch((e) => log.push("load " + show(e)));
import("./x.js", { with: { type: "json" } }).catch((e) => log.push("attrs " + show(e)));
import("y", 5).catch((e) => log.push("options " + show(e)));
import("y", { with: 1 }).catch((e) => log.push("with " + show(e)));
import("y", { with: { a: 1 } }).catch((e) => log.push("value " + show(e)));
import({ toString() { throw new Error("toString"); } }).catch((e) => log.push("specifier " + show(e)));
import("z", undefined,).catch((e) => log.push("trailing comma " + show(e)));
Promise.resolve().then(() => log.push("tick1")).then(() => log.push("tick2"));
try {
  await import("w");
} catch (e) {
  log.push("await " + show(e));
}
console.log(log.join("\n"));
