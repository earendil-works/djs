// localeCompare, < and concatenation of short strings (0277): one-byte strings compare
// by bytes (the result is the first difference or the length's sign), others through NFC.
const pairs = [["apple", "banana"], ["apple", "apples"], ["apples", "apple"], ["same", "same"], ["", "a"], ["a", ""],
  ["é", "e"], ["Zebra", "apple"], ["naïve", "naive"], ["ŝ", "s"], ["e\u0301", "é"], ["x".repeat(40) + "a", "x".repeat(40) + "b"]];
for (const [a, b] of pairs) console.log(JSON.stringify(a), JSON.stringify(b), a.localeCompare(b), a < b, a > b, a + b === `${a}${b}`);
const words = ["pear", "Äpfel", "apple", "fig", "kiwi", "plum", "date", "lime", "banana", "éclair", "ŝip"];
console.log(words.slice().sort((x, y) => x.localeCompare(y)).join(","));
const big = "ab".repeat(30);
console.log((big + "c").localeCompare(big + "d"), ("pre" + "fix") === "prefix", ("é" + "x").length, ("ŝ" + "x" + "y").length);
console.log(" left".padEnd(9, "-") + "|", "7".padStart(4, "0"), "a,b,,c".split(","), "lines\nof\ntext".split("\n").map((l) => l.slice(1)));
