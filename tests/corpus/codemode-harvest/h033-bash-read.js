// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "src/file1.json\nsrc/dir7/dir3/file2.json\nlib/dir4/file3.rs\nlib/dir7/dir3/file4.ts\nsrc/dir6/dir2/file5.rs\nsrc/file6.js"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "Iota east omega kappa entry record\n- East beta delta\n- Zeta alpha status status eta build\n# Blue west status\n# Kappa item done record green green\nBeta alpha record done done done east\n1. Entry gamma eta item record iota\n## Done alpha blue west theta eta item\nBlue blue ready zeta entry iota"
  },
  {
   "ok": true,
   "value": "Zeta build iota status zeta\n- Kappa beta south gamma omega item build omega\nIota status\n## Check omega west epsilon delta epsilon entry west\nEta record iota entry theta theta epsilon\nKappa red ready item\n1. Blue kappa eta build blue gamma\nZeta epsilon south west blue north ready\n- South build ready epsilon done gamma south check\n- Blue green\n- Red west green epsilon check blue record"
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
const files = (await tools.bash({command: "ls pi-tubadal-*.bige", timeout: 10})).output.trim().split("\n");
let out = "";
for (const f of files) {
  const t = await tools.read({path: `/qas/${f}.qop`, offset: null, limit: null});
  const lines = t.split("\n").filter(l => !l.startsWith("  > "));
  out += `\n\n======== ${f}\n` + lines.join("\n");
}
return out;
})(fake);
