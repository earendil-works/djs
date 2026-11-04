// 0199: a long chain: nested loops, a closure over a loop variable in its TDZ-free scope,
// a helper reading a const declared after the loops (in its TDZ at every dump), 60 awaits.
async function main() {
  const log = [];
  const note = (s) => log.push(s + ":" + LATE);
  let total = 0;
  for (let i = 0; i < 9; i++) {
    for (const j of ["a", "b", "c", "d", "e"]) {
      const v = await later(i * 10 + j.charCodeAt(0));
      total += v;
      if (v % 7 === 0) log.push(() => i + j);
      try {
        if (j === "c") total += await later(1);
      } finally {
        total++;
      }
    }
  }
  const LATE = "late";
  note("end");
  print(total, log.length, typeof log[0] === "function" ? log[0]() : log[0], log.at(-1));
}
main();
