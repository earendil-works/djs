let s = "abc";
s += 1;
s += true;
console.log(s, s.length, s[0], s[100], "€uro".length);
let long = "";
for (let i = 0; i < 100; i++) long += i;
console.log(long.length, long[150]);
console.log(`a${1 + 1}b${"c"}${null}${undefined}`, "x" + 1 + 2, 1 + 2 + "x");
console.log("a" < "b", "B" < "a", "10" < "9", 10 < 9, "10" < 9);
