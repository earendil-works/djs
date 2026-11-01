// String building, concatenation and methods (string_build*, string_length).
// bench: expect 839087
let s = "";
for (let i = 0; i < 200_000; i++) s += String.fromCharCode(97 + (i % 26));
let parts = [];
for (let i = 0; i < 50_000; i++) parts.push("item " + i);
const joined = parts.join(", ");
let count = 0;
for (let i = 0; i < 200; i++) count += joined.indexOf("item " + (i * 250)) > 0 ? 1 : 0;
return s.length + joined.length + joined.split(", ").length + count;
