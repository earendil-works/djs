// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "- Beta status north\n# Theta east ready\n# Kappa east alpha value status check north\nEta east entry record check"
  },
  {
   "ok": true,
   "value": "}done delta gamma beta\n  return done entry\nlet beta check epsilon\nfunction eta south\n  return eta beta ready\nfunction epsilon north east entry\nfunction check\n  if (done test lambda theta\n}done\n  if (east check build check\n  return ready lambda west green\n  if (entry\nlet test ready"
  },
  {
   "ok": true,
   "value": "function omega build kappa\n  return kappa ready\n}record blue test\nexport delta blue epsilon\nconst value theta alpha\nlet south gamma\nconst item test north omega\nfunction epsilon entry zeta gamma\n  if (test check kappa\nfunction north blue value\n}item eta test\nexport gamma ready lambda\n  return done check eta"
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
const [a,b,c]=await Promise.all([
tools.read({path:"qine/fopa1.md",offset:1015,limit:45}),
tools.read({path:"tiv/ror2/fopa3.js",offset:null,limit:null}),
tools.read({path:"tiv/ror4/fopa5.js",offset:null,limit:null})]);
return [a,b,c].join("\n=====\n")
})(fake);
