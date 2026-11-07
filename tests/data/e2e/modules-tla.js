// mode: module
import { slow } from "./slow.js";
import { fast } from "./fast.js";
import data from "./data.js";
console.log("main", slow, fast, data.n);
const parts = [];
for await (const p of [Promise.resolve("a"), "b"]) parts.push(p);
console.log("for await", parts.join(""));
export const late = await new Promise((r) => r("late"));
console.log(late, (await import("./fast.js")).fast);
try {
  await import("./rejects.js");
} catch (e) {
  console.log("rejects:", e);
}
try {
  await import("./rejects.js");
} catch (e) {
  console.log("again:", e);
}
// file: slow.js
console.log("slow starts");
let n = 0;
for (let i = 0; i < 3; i++) n += await i;
export const slow = n;
console.log("slow ends");
// file: fast.js
console.log("fast starts");
await null;
export const fast = "fast";
console.log("fast ends");
// file: data.js
export default { n: await Promise.resolve(42) };
// file: rejects.js
await 0;
throw new RangeError("rejected late");
