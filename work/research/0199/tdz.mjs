import { toPlain } from "/Users/mitsuhiko/Development/djs/.worktrees/plainjs/work/scratch/0199/plain.mjs"; import fs from "node:fs";
let d = 0, c = 0, acts = 0, merged = 0;
for (const f of fs.readdirSync("today").filter((f) => f.endsWith(".dump.js"))) {
  let r; try { r = toPlain(fs.readFileSync("today/" + f, "utf8"));
  } catch { continue; } if (r.stats.tdzCells) { d++; c += r.stats.tdzCells; } acts += r.stats.activations; merged += r.stats.merged;
}
console.log("dumps with TDZ shared vars", d, "vars", c, "activations", acts, "merged factories", merged);
