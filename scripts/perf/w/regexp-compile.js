// RegExp construction (0224): the patterns of djs-bench's regexp/compile group, each
// compiled anew (new RegExp is not cached).
// bench: expect 60000
const pats = [
  ["\\d+", ""],
  ["[\\w.+-]+@[\\w-]+\\.[\\w.]+", ""],
  ["https?://([^/\\s]+)(/[^\\s?#]*)?", ""],
  ["(?<y>\\d{4})-(?<m>\\d{2})-(?<d>\\d{2})", ""],
  ["\\bfox\\b", "i"],
  ["^\\s+|\\s+$", "g"],
];
let n = 0;
for (let i = 0; i < 10000; i++) {
  for (const [p, f] of pats) {
    if (new RegExp(p, f).source.length > 0) n++;
  }
}
return n;
