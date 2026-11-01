// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "1. Value omega gamma\nIota west lambda epsilon theta beta omega\nIota green item status item done done\nSouth status beta blue\n1. Beta done north north lambda alpha green west"
  },
  {
   "ok": true,
   "value": "# South blue west south zeta west status value\n- Zeta ready zeta gamma\n## East check south epsilon red delta green\n## South check status iota ready done west\n## Status check\n# Entry zeta theta epsilon red west blue status\n# Epsilon blue ready entry\n# Done green\n# Eta north build north delta zeta status"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "src/file9.ts:175: kappa zeta east\nsrc/file6.ts:15: gamma record alpha build entry south\nsrc/file7.ts:196: build alpha theta south\nsrc/file7.ts:60: done item kappa\nsrc/file1.ts:177: epsilon beta epsilon\nsrc/file9.ts:34: zeta done west\nsrc/file7.ts:144: build entry\nsrc/file2.ts:57: zeta beta delta\nsrc/file8.ts:7: gamma value theta theta record\nsrc/file6.ts:233: north blue gamma theta theta check\nsrc/file8.ts:375: gamma theta theta zeta\nsrc/file6.ts:350: lambda kappa zeta zeta\nsrc/file2.ts:175: zeta omega check south"
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
const r = await Promise.allSettled([
 tools.read({path:"NIFOM.md", offset:null, limit:null}),
 tools.read({path:"qod1/VIBIDA.md", offset:null, limit:null}),
 tools.bash({command:"nuda -n '^## \\|^### Q\\|^- \\*\\*Q\\|^### P\\|^Q[0-9]' BEGUMOGOD.md | lege -100", timeout:null})
]);
return r.map(x=>x.value ?? String(x.reason)).join("\n\n=====\n\n");
})(fake);
