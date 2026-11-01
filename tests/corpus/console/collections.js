// Map, Set and friends have no enumerable own properties: JSON shows {}.
console.log(new Map([["a", 1]]), new Set([1, 2]));
console.log({ m: new Map(), s: new Set() });
console.log(new WeakMap(), new WeakSet());
console.log(Promise.resolve(1));
console.log(/re+gex/gi);
console.log(new Uint8Array([1, 2, 3]));
console.log(new ArrayBuffer(4));
// Spread them to see the contents.
console.log([...new Map([["a", 1]])], [...new Set([1, 2])]);
