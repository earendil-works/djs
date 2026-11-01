// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "1. Omega theta item\n- Blue south value entry item north theta\n## West red alpha theta\n## Omega done omega value build green\n## Item south status east build west\n# Omega build gamma red delta entry\n# Omega lambda item epsilon red item test blue\n# North item north epsilon test west kappa iota\n1. East entry beta epsilon build west south\nEntry iota gamma check"
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
const c = await tools.read({path: "/der/venu1.md", offset: 1, limit: 2000}); return [c.slice(0,120), c.slice(-200)];
})(fake);
