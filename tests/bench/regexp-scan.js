// Regex scans over large text (0056): a property-escape loop over a 400k-unit string,
// split and replace on unmodified RegExps, and test in a loop over lines.
// bench: expect 166892
let text = "";
for (let i = 0; i < 4000; i++) text += "Grüße, Привет! word" + i + " 漢字 abc;def\n";
const letters = text.match(/\p{L}+/gu).length;
const lines = text.split(/\r?\n/);
let tests = 0;
for (const l of lines) if (/^Gr\w+, (\S+)/u.test(l)) tests++;
const replaced = text.replace(/\s+/g, " ").length;
const big = "QUJD".repeat(100_000);
const b64 = /^[A-Za-z0-9+/]+={0,2}$/.test(big + "==") ? 1 : 0;
return letters + lines.length + tests + replaced + b64;
