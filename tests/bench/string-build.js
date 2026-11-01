// String building like test262's regExpUtils buildString (0056): an element-store loop,
// String.fromCodePoint.apply with 10k arguments, += of the chunks; Math.max.apply.
// bench: expect 335548.95
const codePoints = [];
let result = "";
let length = 0;
for (let cp = 0x400; cp < 0x400 + 200_000; cp++) {
  codePoints[length++] = cp % 0x10000 === 0xd800 ? 0x41 : cp;
  if (length === 10_000) {
    result += String.fromCodePoint.apply(null, codePoints);
    codePoints.length = length = 0;
  }
}
result += String.fromCodePoint.apply(null, codePoints);
const nums = [];
for (let i = 0; i < 1000; i++) nums.push((i * 7919) % 1000);
let m = 0;
for (let i = 0; i < 50; i++) m += Math.max.apply(null, nums);
return result.length + m / 1000 + String.fromCharCode.apply(null, nums.slice(0, 13)).length;
