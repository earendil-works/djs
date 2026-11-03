// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "let epsilon kappa\n  return build omega west\nlet iota kappa kappa\nlet item\nconst test\nconst north west epsilon\nconst west\nfunction east iota\n  if (test\nconst record\n}east\nconst green epsilon value\n  return kappa"
  },
  {
   "ok": true,
   "value": "const iota test\n  if (green blue north\n}status omega\nlet status ready green south\nfunction north\n  if (red zeta kappa check\nconst delta west beta\n  if (ready ready ready beta\nconst ready gamma epsilon omega\nconst done red gamma\n}south item epsilon epsilon\nexport alpha east south gamma\nconst beta epsilon\nexport east blue omega red"
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
const a = await tools.read({path:"bef1/remu2.nok", offset:44, limit:5});
const b = await tools.read({path:"bef1/remu3.py", offset:28, limit:5});
return a + "\n---\n" + b;
})(fake);
