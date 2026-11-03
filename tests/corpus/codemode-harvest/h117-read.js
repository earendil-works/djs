// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "let item lambda test\n}gamma\nconst theta\n}status value item iota\nfunction lambda zeta north alpha"
  },
  {
   "ok": true,
   "value": "  if (beta epsilon epsilon\nlet item status\nconst theta south done\n  return done item\nfunction theta done entry test\n  if (omega alpha alpha\nconst alpha blue omega\n  return south zeta build kappa\nconst test iota"
  },
  {
   "ok": true,
   "value": "const build\n  if (beta\nfunction west west alpha\nlet check theta omega\nconst zeta ready\nfunction zeta red red\nexport green ready record\n}west value item epsilon\nexport item\n}theta ready item\n}build"
  },
  {
   "ok": true,
   "value": "}kappa blue blue entry\n}status entry record south\nfunction entry iota alpha\n  if (south\nlet gamma omega blue\n}epsilon\nconst east alpha alpha"
  },
  {
   "ok": true,
   "value": "export zeta theta item record\nfunction zeta iota\n  return red zeta iota alpha\nexport record kappa south green"
  },
  {
   "ok": true,
   "value": "function check done south\nexport entry red\nlet kappa\n  if (blue"
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
const p='dir1/src/file2.rs';
const parts = await Promise.all([[300,20],[370,80],[615,60],[700,30],[920,20],[1140,70]].map(([o,l])=>tools.read({path:p,offset:o,limit:l})));
return parts.join("\n=====\n");
})(fake);
