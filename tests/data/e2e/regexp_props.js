// RegExp completion (0050): property escapes, loops over one character (STAR), shared
// literal programs, and the split/replace fast paths, on inputs small enough for the
// fuel-1 suspension mode.
console.log("Grüße, Привет! 漢字 42".match(/\p{L}+/gu).join("|"), /\p{Script=Cyrillic}/u.test("ж"), /^\P{ASCII}+$/u.test("é漢"));
console.log(JSON.stringify("a😀b👍🏽".match(/\p{Emoji_Presentation}/gu)), /\p{Lu}/iu.test("q"), /[\p{Lu}--[A-Z]]/v.test("A"));
const csv = "id,name ,  value\n1, a ,10\n2,b,  20\n";
const rows = csv.trim().split(/\r?\n/).map((l) => l.split(/\s*,\s*/));
console.log(JSON.stringify(rows), csv.replace(/\s+/g, " "), csv.replace(/(\d+)/g, "<$1>").length);
console.log("<a><b>".match(/<.*?>/g).join(""), "<a><b>".match(/<.*>/)[0], "x".repeat(50).replace(/x*$/, "!"), /^[^@]+@[^@]+$/.test("me@example.org"));
const words = [];
for (let i = 0; i < 3; i++) words.push(/(\w+)@(\w+)/.exec("contact: joe@example")[i]);
console.log(words.join(" "), "aaa".split(/a*?/).length, "a,b,,c".split(/,/, 3).join("|"));
