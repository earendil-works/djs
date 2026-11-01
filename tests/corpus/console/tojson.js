// toJSON is honored, including on nested values and with the key argument.
console.log({ toJSON() { return "replaced"; } });
console.log({ a: { toJSON(key) { return "key=" + key; } } });
console.log([{ toJSON(key) { return "index=" + key; } }]);
console.log({ toJSON() { return undefined; } });
console.log({ toJSON() { return { nested: [1] }; } });
const d = new Date(Date.UTC(2020, 1, 29, 12, 30, 45, 123));
console.log(d);
console.log({ when: d });
console.log(new Date(NaN));
