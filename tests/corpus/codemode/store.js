// store/load keep JSON copies.
console.log(load("missing"));
store("count", 1);
store("obj", { list: [1, 2], when: new Date(0) });
const obj = load("obj");
obj.list.push(3);
console.log(load("count"), load("obj"), obj.list);
store("count", undefined);
console.log(load("count"));
try {
  store(1, "x");
} catch (e) {
  console.log(e);
}
try {
  const c = {};
  c.c = c;
  store("cyclic", c);
} catch (e) {
  console.log(e);
}
try {
  store("fn", () => 1);
} catch (e) {
  console.log(e);
}
