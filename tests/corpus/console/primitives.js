// console.log formats strings as they are and everything else with JSON.stringify,
// falling back to String() when JSON has no representation.
console.log("plain string");
console.log(1, 1.5, -1, 0, -0, 1e21, 1e-7, 2 ** 53, -(2 ** 31));
console.log(NaN, Infinity, -Infinity);
console.log(true, false, null, undefined);
console.log(10n, -5n);
console.log(Symbol("desc"), Symbol(), Symbol.iterator);
console.log();
console.log("");
console.log("a", "", "b");
