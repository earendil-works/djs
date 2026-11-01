// The globals a script can see (sorted, so the order of definition does not matter).
const names = Object.getOwnPropertyNames(globalThis).sort();
console.log(names.filter((n) => ["console", "text", "exit", "tools", "ALL_TOOLS", "store", "load"].includes(n)));
console.log(typeof print, typeof setTimeout, typeof require, typeof process, typeof fetch);
