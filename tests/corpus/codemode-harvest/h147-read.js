// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "let eta kappa\nlet build\nlet ready test iota"
  },
  {
   "ok": true,
   "value": "  return gamma check\n  return blue eta build\nexport beta epsilon south\nfunction north south\nlet zeta build lambda\nfunction done west eta\nexport alpha west"
  },
  {
   "ok": true,
   "value": "const check\n}red\n  return ready"
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
["dir1/dir2/src/lib.rs",1100,20],
["dir1/dir3/src/dir4/file5.rs",1,70],
["dir1/core/src/file6.rs",1010,45],
].map(async ([path,offset,limit])=>text(await tools.read({path,offset,limit}))));
})(fake);
