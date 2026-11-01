// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "iota build test west ready iota status\ngamma omega green west 3341\nvalue gamma 7569\ndone check theta epsilon alpha zeta red\neta check omega lambda record build",
    "truncated": false
   }
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
 "dl": {
  "url": "beta ready iota epsilon value delta test green entry kappa east green value alpha epsilon iota epsilon test test value south theta green item west gamma entry item eta done item iota epsilon check eas",
  "method": "eps",
  "headers": {},
  "expires_at": "alpha alpha red south va"
 }
};
const realLoad = load;
const fakeLoad = (key) => {
  const own = realLoad(key);
  if (own !== undefined) return own;
  return key in stored ? JSON.parse(JSON.stringify(stored[key])) : undefined;
};
return await (async (tools, load) => {
const s = load("dl");
const h = Object.entries(s.headers||{}).map(([k,v])=>`-H ${JSON.stringify(k+": "+v)}`).join(" ");
const r = await tools.bash({command:`zatez -p /qat/qon1 && dadu -sSf -X ${s.method} ${h} ${JSON.stringify(s.url)} -o /qat/qon1/givu2.fepe && qig ravopevete /qat/qon1/givu2.fepe > /qat/qon1/givu3.md && wc -l /qat/qon1/givu3.md`, timeout:120});
return r.output;
})(fake, fakeLoad);
