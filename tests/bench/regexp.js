// Regex test, exec and replace (regexp_ascii, regexp_replace).
// bench: expect 32000
const line = "2024-01-15 john.doe@example.com GET /a/b?q=1 200 1234ms";
let n = 0;
for (let i = 0; i < 20_000; i++) {
  if (/(\d{4})-(\d{2})-(\d{2})/.test(line)) n++;
  const m = /([\w.]+)@([\w.]+)/.exec(line);
  if (m && m[2] === "example.com") n++;
  n += line.replace(/\d/g, "#").length > 50 ? 0 : 1;
}
return n * 0.8;
