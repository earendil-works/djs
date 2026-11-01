// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
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
  fake[name] = async (args) => {
    JSON.stringify(args);
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    if (!r.ok) throw new Error(r.value);
    return r.value;
  };
}
return await (async (tools) => {
await Promise.allSettled([
["dir1/core/src/dir2/dir3/file4.rs",1,160],
["dir1/utils/dir5/src/lib.rs",107,100],
].map(async ([path,offset,limit]) => text(await tools.read({path,offset,limit}))));
})(fake);
