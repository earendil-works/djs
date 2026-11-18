const all = [];
for (let c = 0; c < 256; c++) all.push(String.fromCharCode(c));
const s = all.join("");
console.log(JSON.stringify(s.toLowerCase()) === JSON.stringify([...s].map((c) => c.toLowerCase()).join("")));
console.log(escape(s.toLowerCase()));
console.log(escape(s.toUpperCase()));
console.log(escape(s.replace(/[µßÿ]/g, "").toUpperCase()));
for (let c = 0; c < 256; c++) { const t = String.fromCharCode(c); const r = (t + "x" + t).trim(); if (r !== "x" && r.length !== 3) console.log("bad", c); }
console.log([..." \t\n\v\f\r\u00a0\u0085x\u00a0 "].map((c) => c.charCodeAt(0)).join(), JSON.stringify(" \t\n\v\f\r\u00a0\u0085x\u00a0 ".trim()), JSON.stringify("\u0085x".trimStart()), JSON.stringify("  ".trim()), JSON.stringify("".trim()));
console.log("ÀÉÎõü×÷".toLowerCase(), "àéîõü×÷þ".toUpperCase(), "ABC".toLowerCase() === "abc", "abc".toLowerCase());
console.log((255).toString(), (255).toString(16), (-0).toString(), (1e21).toString(), (0.1).toString(10), (123.456).toString(undefined), (NaN).toString(), (2**53).toString());
console.log("hello world, this is a line of code".slice(3, 9), "x".repeat(300).slice(10, 290).length, "abc".substring(1), "\u1234abc".slice(1, 3));
console.log(["5".padStart(3, "0"), "abc".padEnd(10, "12"), "abc".padStart(2), "x".padStart(5), "é".padEnd(4, "ü"), "a".padStart(4, "\u1234b"), "\u1234".padEnd(3, "-"), "ab".padStart(8, ""), "ab".padEnd(70000, "xyz").length, "ab".padStart(5, { toString() { return "*"; } }), "q".padEnd(4, null), "q".padStart(3, 7)].map((s) => JSON.stringify(s)).join(" "));
