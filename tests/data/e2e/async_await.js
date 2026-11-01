// mode: body
// Async functions, arrows and methods, await ordering against plain promise jobs, and
// top-level await in a codemode body (0041).
const log = [];
async function f(x) {
  log.push("f start " + x);
  const v = await x;
  log.push("f got " + v);
  return v * 2;
}
const g = async (x) => (await f(x)) + 1;
const o = {
  async m(a = 1) {
    try {
      await Promise.reject(new Error("boom"));
    } catch (e) {
      return "caught " + e.message + a;
    } finally {
      log.push("fin");
    }
  },
};
async function thrower() {
  await null;
  throw new TypeError("bad");
}
async function early(a = (() => { throw "param"; })()) {}
f(1).then((v) => log.push("f1 " + v));
g(Promise.resolve(5)).then((v) => log.push("g " + v));
o.m().then((v) => log.push(v));
thrower().catch((e) => log.push("thrower " + e.message));
early().catch((e) => log.push("early " + e));
Promise.resolve().then(() => log.push("t1")).then(() => log.push("t2")).then(() => log.push("t3"));
log.push("sync");
// Top-level await: the body is itself an async function.
const r = await g(10);
console.log(r, log.join(", "));
// Thenables are adopted; await in loops, conditionals and argument lists.
const thenable = { then(res) { res("thenable"); } };
console.log(await thenable);
let sum = 0;
for (const p of [1, Promise.resolve(2), 3]) sum += await p;
for (let i = 0; i < 3; i++) if (i % 2) sum += await i;
console.log(sum, Math.max(await 4, await Promise.resolve(5)));
// Errors thrown by awaited rejections are catchable; finally runs.
try {
  await thrower();
} catch (e) {
  console.log("caught", e.message);
} finally {
  console.log("finally");
}
// Function properties of async functions.
console.log(f.name, g.name, o.m.name, f.length, typeof f.prototype, String(f(0) instanceof Promise));
console.log(Object.getPrototypeOf(f) === Object.getPrototypeOf(g), Object.prototype.toString.call(f));
// `await` is an identifier in non-async functions inside the body.
function plain() { var await = 3; return await; }
console.log(plain());
return "done";
