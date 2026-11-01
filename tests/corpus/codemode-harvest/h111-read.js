// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "# South south blue\n## South check item zeta\nLambda ready east lambda build"
  },
  {
   "ok": true,
   "value": "# Check ready done kappa\n## North ready theta omega\n- Entry build iota iota north iota red\n1. Check theta gamma kappa iota north\n## Record lambda\n## Epsilon zeta check south west record done\n## Eta value omega lambda entry blue east record\n## Alpha lambda beta\n- Gamma check done\n## Kappa eta\nCheck delta build red\n- Green south ready eta omega value ready delta\n- Omega west gamma"
  },
  {
   "ok": true,
   "value": "# Gamma red zeta south beta blue\n## Record east item west ready done east\n1. Eta red value status build iota\nAlpha iota red south iota done iota\n## Kappa north\nGreen delta value status check build entry gamma\n1. Eta west status gamma\n- West lambda entry zeta blue red record south\n1. Test west south north lambda\n# West delta alpha east build south value\n1. Eta epsilon entry red entry north entry\n- Gamma done theta delta omega\nEta omega value kappa iota done lambda gamma\nSouth blue"
  },
  {
   "ok": true,
   "value": "Status beta\n- Ready zeta lambda delta green kappa\nEntry north record entry\n# Record green south test alpha status west\n- North south east zeta beta\n- Kappa blue item entry"
  },
  {
   "ok": true,
   "value": "- Value kappa lambda\n# Delta red epsilon omega status value value red\n- Status value blue beta\n# Green north east done\nAlpha omega beta\nLambda record iota\nAlpha north omega\n- Beta build done omega south check test\n1. Beta iota beta"
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
const r = await Promise.all(["docs/file1.md","README.md","docs/file2.md","docs/file3.md","docs/file4.md"].map(p=>tools.read({path:p,offset:null,limit:null})));
return r.join("\n\n=========\n\n");
})(fake);
