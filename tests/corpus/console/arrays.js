// Sparse arrays print holes as null; there is no truncation of long values.
const sparse = [1, , 3];
sparse[6] = 7;
console.log(sparse);
console.log(new Array(3));
const withProps = [1, 2];
withProps.extra = "ignored";
console.log(withProps);
console.log(Array.from({ length: 150 }, (_, i) => i));
console.log({ long: "y".repeat(200) });
const deep = [];
let cur = deep;
for (let i = 0; i < 30; i++) {
  const next = [];
  cur.push(i, next);
  cur = next;
}
console.log(deep);
