// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "function theta value item\n  if (south\n}west alpha\nexport zeta item lambda eta\n  if (item eta entry\n  if (blue\nfunction delta north zeta zeta\nlet done omega value build\nconst east lambda done west\n  return blue"
  },
  {
   "ok": true,
   "value": "function kappa north iota\n  return lambda iota blue entry\nexport gamma entry east\n  return west kappa beta\n  if (alpha status south kappa\nlet alpha green blue test\n  return status test record\nconst delta done"
  },
  {
   "ok": true,
   "value": "export entry lambda status\n  if (status eta delta\n}value value\n  if (east west\n}east record check iota\nlet kappa\n  return west green record\n}blue gamma south\nexport test"
  },
  {
   "ok": true,
   "value": "}green value\nconst ready blue ready\nfunction test blue\nfunction epsilon"
  },
  {
   "ok": true,
   "value": "let epsilon kappa omega item\nlet west epsilon build\n  return record\nfunction blue done\nfunction delta\nlet build\nfunction gamma gamma"
  },
  {
   "ok": true,
   "value": "  return blue lambda west north\nlet done ready\n  if (delta record beta\n  if (theta done item epsilon\n}blue ready check"
  },
  {
   "ok": true,
   "value": "- Build value\nEpsilon done\nGreen done build test"
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
for (const p of ["src/dir1/file2.js","src/dir1/file3.js","src/dir1/file4.js","src/core/file5.js","src/dir6/file7.js","tests/file8.js","docs/file9.md"]) {text(p);text(await tools.read({path:p,offset:p.endsWith("dazi/pisu5.js")?114:undefined,limit:p.endsWith("dazi/pisu5.js")?30:undefined}));}
})(fake);
