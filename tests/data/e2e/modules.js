// mode: module
import { count, inc, Counter } from "./counter.js";
import * as ns from "./counter.js";
import def, { later } from "./cycle.js";
import { a as reA, all } from "./reexport.js";
console.log("main starts", count, typeof ns, Object.keys(ns).join());
inc();
console.log(count, ns.count, new Counter().value);
console.log(def(), later(), reA, Object.keys(all).join());
try { ns.count = 5; } catch (e) { console.log(e.name); }
try { count = 5; } catch (e) { console.log(e.name); }
console.log(Object.getOwnPropertyNames(ns).join(), String(ns[Symbol.toStringTag]));
let s = 0;
for (let i = 0; i < 100; i++) s += i;
export const total = s;
await0();
function await0() { console.log("hoisted", typeof await0); }
// file: counter.js
console.log("counter runs");
export let count = 0;
export function inc() { count++; }
export class Counter { value = count * 10; }
export default "the default";
// file: cycle.js
import { back } from "./cycle2.js";
console.log("cycle runs", back());
export default function () { return "anonymous default"; }
export function later() { return "later"; }
// file: cycle2.js
import def from "./cycle.js";
console.log("cycle2 runs", typeof def);
export function back() { return "back " + def(); }
// file: reexport.js
export { count as a } from "./counter.js";
export * as all from "./counter.js";
