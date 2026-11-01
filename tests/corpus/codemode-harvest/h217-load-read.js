// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "- Theta omega zeta\n- Omega entry zeta delta south omega item green\n- Red epsilon blue theta\nAlpha north ready record\n- Value epsilon check east entry status\n## Status value record item ready blue theta\nDone south\n## Iota red record kappa omega entry epsilon\nItem done epsilon\n## Ready ready green entry check item build gamma"
  },
  {
   "ok": true,
   "value": "# Entry green value value eta\n1. South done west iota\n## Test check epsilon west north entry beta eta\n1. Delta beta status\n## Entry gamma test\n1. Record kappa alpha delta beta iota green\nBlue blue blue lambda blue check\nGamma alpha theta done omega\n1. Zeta build west iota test\n1. Delta gamma beta\n# Ready lambda eta theta done build\n# Gamma eta omega\n# Gamma red\n- Eta ready iota"
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
const stored = {
 "doc-settings.md": "theta alpha delta east entry red record red item ready ready north east zeta east gamma beta omega blue kappa entry record lambda theta eta green blue blue entry item gamma check epsilon lambda record",
 "doc-qug.md": "value test done ready kappa kappa item beta build east ready north test delta theta delta entry check theta record blue status ready entry iota entry zeta zeta check entry beta test red delta kappa ch"
};
const realLoad = load;
const fakeLoad = (key) => {
  const own = realLoad(key);
  if (own !== undefined) return own;
  return key in stored ? JSON.parse(JSON.stringify(stored[key])) : undefined;
};
return await (async (tools, load) => {
text(await tools.read({path:"filoviso/qag1/soto/gasu2.md"}));
text(await tools.read({path:"filoviso/qag1/soto/gasu3.md"}));
text(load("doc-settings.md"));
text(load("doc-qug.md"));
})(fake, fakeLoad);
