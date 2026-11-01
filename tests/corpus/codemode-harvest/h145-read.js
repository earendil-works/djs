// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "  return beta\n  if (entry\nlet delta check\nconst omega\n}ready green"
  },
  {
   "ok": true,
   "value": "}north\n  return value theta lambda status\nlet theta check entry eta\nexport gamma lambda\nconst record check epsilon\nconst iota\nexport alpha south\nconst kappa record west"
  },
  {
   "ok": true,
   "value": "let theta south gamma\nconst record\n  if (lambda check status\nlet entry green omega\n}item epsilon done\nexport build delta\n  if (kappa gamma status blue\nconst delta east iota\nlet theta red\n  return red red zeta\n  if (south check\nconst epsilon delta"
  },
  {
   "ok": true,
   "value": "}omega theta\n  return blue\n}done value beta\nexport lambda check ready\n  if (north west\nlet west delta"
  },
  {
   "ok": true,
   "value": "  return zeta\n  if (value theta\n}build alpha record\nconst kappa test epsilon"
  }
 ]
};
const fake = Object.create(null);
for (const name of Object.keys(results)) {
  const queue = results[name];
  fake[name] = async (args) => {
    JSON.stringify(args);
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    if (!r.ok) throw new Error(r.value);
    return r.value;
  };
}
return await (async (tools) => {
const base="/home/user/dir1/dir2/koni/dir3/";
const files=["src/dir4/file5.ts","src/dir6/file7.ts","src/dir6/file8.ts","src/dir6/file9.ts","src/dir4/file9.ts"];
const r=await Promise.all(files.map(f=>tools.read({path:base+f,offset:null,limit:null})));
return r.map((t,i)=>`=== ${files[i]}\n${t}`).join("\n");
})(fake);
