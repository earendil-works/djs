// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "  if (test\nconst epsilon\n  if (zeta zeta green south\nfunction omega beta iota\n  return west zeta zeta\nconst status green\nfunction record north zeta\nfunction north ready\nexport status value alpha\nlet done zeta\nexport entry north lambda ready"
  },
  {
   "ok": true,
   "value": "let beta omega blue\nexport value ready theta\nfunction theta item\nexport east delta\nexport delta east\n  if (lambda\n  return lambda\n  if (epsilon\n  if (delta\nlet item item eta\nconst item status south"
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
await Promise.allSettled(["src/dir1/file2.js","src/dir3/file4.js"].map(async path => text({path,content:await tools.read({path})})));
})(fake);
