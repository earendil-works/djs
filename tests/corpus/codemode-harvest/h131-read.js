// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "function south\nexport zeta theta\n  return status east\nlet lambda\nfunction theta test ready delta\nlet status alpha\nfunction north zeta\nlet gamma epsilon\n}kappa\nexport kappa epsilon blue iota\nconst status south\nfunction check gamma gamma"
  },
  {
   "ok": true,
   "value": "export value epsilon red green\n  return build entry kappa value\nexport test build\nconst build ready north record\nconst eta record lambda\nlet green\nexport green\nconst test\n  if (green\nlet item lambda red value"
  },
  {
   "ok": true,
   "value": "  return gamma eta iota\n}delta\nlet omega ready done\n}status entry red eta\n}lambda\n  if (omega"
  },
  {
   "ok": true,
   "value": "  if (item delta\n  if (south build gamma\n  if (omega zeta\n  if (epsilon\n  return iota north red status\n  if (theta lambda north\nlet test alpha test iota\n  if (west eta"
  },
  {
   "ok": true,
   "value": "let kappa red test\n}delta zeta\n  return zeta status item\n  return blue lambda value test\nconst blue check\nconst iota\nexport green"
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
for (const path of ["src/dir1/file2.js","src/core/file3.js","src/dir1/file4.js","src/dir1/file5.js","src/dir1/file6.js"]) text({path,content:await tools.read({path})});
})(fake);
