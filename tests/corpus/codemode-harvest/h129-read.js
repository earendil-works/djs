// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "function check done\n}iota value omega\n  return done eta red\nlet green east\nexport lambda item west\nlet done north record value\n  return west\n  if (epsilon check entry entry\nlet epsilon\nfunction build done lambda value\nlet ready zeta"
  },
  {
   "ok": true,
   "value": "export beta delta status test\n}ready eta\n  return north\nfunction gamma\nfunction done\n}done build north blue\nfunction green\nconst omega green west\n}green theta lambda\nexport green kappa done test\n  if (blue gamma\nexport west north delta"
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
await Promise.allSettled([
["dir1/core/src/dir2/dir3/file4.rs",1,160],
["dir1/utils/dir5/src/lib.rs",107,100],
].map(async ([path,offset,limit]) => text(await tools.read({path,offset,limit}))));
})(fake);
