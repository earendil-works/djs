// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "}theta record\nconst test status check\nexport done check\nexport done east"
  },
  {
   "ok": true,
   "value": "  if (west kappa kappa\nfunction north\nlet build iota omega value\n}value zeta value"
  },
  {
   "ok": true,
   "value": "function lambda theta zeta status\n  return item west check\n  return value alpha alpha item\n  if (entry build\n  if (iota build test check\n}alpha omega\nfunction item item\nexport iota record\n  if (green entry status\n  return north alpha west build"
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
await Promise.allSettled(['dir1/src/dir2/file3.js','dir1/tests/file4.js','dir1/tests/file5.js'].map(async path=>text({path,content:await tools.read({path})})));
})(fake);
