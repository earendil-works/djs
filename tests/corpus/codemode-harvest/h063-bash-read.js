// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "# Item status status\n## Blue delta kappa red theta red\n## Zeta delta value south build\n# Theta check value\n## Beta alpha entry status entry\n# Gamma south west check record lambda eta build\n1. Ready entry iota theta beta epsilon value delta\n1. Iota epsilon status epsilon eta north"
  },
  {
   "ok": true,
   "value": "# Item build delta entry item done entry\n# Entry kappa check\n# East west epsilon delta iota zeta\n## Alpha west iota check alpha record status\n## Omega beta build done build build"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "src/file6.ts:277: epsilon west test west\nsrc/file9.ts:215: record lambda item check\nsrc/file7.ts:87: build ready done entry lambda\nsrc/file6.ts:128: item done west iota east entry\nsrc/file2.ts:180: iota beta red\nsrc/file5.ts:374: theta iota done blue"
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
 tools.read({path:"kada-vusuvegig-lesiluma.md", offset:null, limit:null}),
 tools.read({path:"kap-dubo.md", offset:null, limit:null}),
 tools.bash({command:"gir safi --fibu; gir saduvu --kagir | kedi '^??'; gir qor --kibirur -8", timeout:20}),
]);
return r.map(x=>x.status==="fulfilled"?(typeof x.value==="string"?x.value:x.value.output):String(x.reason)).join("\n\n=========\n\n");
})(fake);
