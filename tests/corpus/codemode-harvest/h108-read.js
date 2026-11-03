// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "  if (check build\nconst zeta theta\n  return west\n  return north omega\n  return blue zeta\nexport west build check\nconst build zeta gamma status\nconst iota entry\n}gamma\n  if (zeta lambda build\n  if (value blue value\nconst alpha iota eta\n}build theta\nexport done value"
  },
  {
   "ok": true,
   "value": "const alpha ready south epsilon\n  if (value\nfunction value done\n  if (entry\nexport eta eta red\nlet kappa\nconst north\n}south north\n}item green red test\nexport north lambda\nfunction done south test"
  },
  {
   "ok": true,
   "value": "}red eta test\nexport check record record north\n  if (theta delta\n  if (kappa status north\nexport done entry theta record\nconst theta west\n}red north\n}delta theta status\nlet beta\n}build red delta green\n  return west blue green beta\nlet eta test eta\nlet gamma delta value omega"
  }
 ]
};
const fake = Object.create(null);
for (const name of Object.keys(results)) {
  const queue = results[name];
  fake[name] = (args) => {
    try {
      JSON.stringify(args);
    } catch (error) {
      return later(error, true);
    }
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    return r.ok ? later(r.value) : later(new Error(r.value), true);
  };
}
return await (async (tools) => {
const p='dir1/dir2/file3.go';
const parts = await Promise.all([[760,240],[2370,60],[2810,200]].map(([o,l])=>tools.read({path:p,offset:o,limit:l})));
return parts.join("\n=====\n");
})(fake);
