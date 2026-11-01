// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
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
  fake[name] = async (args) => {
    JSON.stringify(args);
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    if (!r.ok) throw new Error(r.value);
    return r.value;
  };
}
return await (async (tools) => {
await Promise.allSettled(['dir1/src/dir2/file3.js','dir1/tests/file4.js','dir1/tests/file5.js'].map(async path=>text({path,content:await tools.read({path})})));
})(fake);
