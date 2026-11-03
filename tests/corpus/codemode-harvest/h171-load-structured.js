// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "nad__blender__execute_blender_code": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "type": "text",
      "text": "(structured)"
     }
    ],
    "structuredContent": {
     "zeta": {
      "value": "green record"
     },
     "beta": {
      "value": "beta alpha kappa"
     }
    }
   }
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
const stored = {
 "monukik": "red iota delta omega status build epsilon beta kappa status green theta delta test eta beta east eta red south ready kappa ready blue gamma ready beta test theta theta beta red check green north value"
};
const realLoad = load;
const fakeLoad = (key) => {
  const own = realLoad(key);
  if (own !== undefined) return own;
  return key in stored ? JSON.parse(JSON.stringify(stored[key])) : undefined;
};
return await (async (tools, load) => {
const r = await tools.nad__blender__execute_blender_code({code: load('regofig')});
return JSON.stringify(r.structuredContent?.result ?? r.structuredContent ?? r).slice(0, 3000);
})(fake, fakeLoad);
