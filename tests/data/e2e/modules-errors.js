// mode: module
import "./a.js";
console.log("main never runs");
// file: a.js
import "./b.js";
export const fromA = 1;
console.log("a never runs");
// file: b.js
import * as a from "./a.js";
import { fromA } from "./a.js";
try { fromA; } catch (e) { console.log("tdz", e.name); }
try { a.fromA; } catch (e) { console.log("ns tdz", e.name); }
console.log(Object.keys(a).join(), "fromA" in a, a.missing);
try { Object.getOwnPropertyDescriptor(a, "fromA"); } catch (e) { console.log("desc", e.name); }
throw new TypeError("boom");
