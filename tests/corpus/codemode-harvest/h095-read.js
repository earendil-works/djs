// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "  return lambda theta east omega\nconst east theta\nfunction gamma\nconst east beta south eta\nconst item east\n  if (red eta item theta\nexport lambda check\nlet beta north\nlet status entry entry\nlet beta eta\n  if (green eta record blue\n}north check\n  if (status zeta beta test\n}red check north check"
  },
  {
   "ok": true,
   "value": "export item theta blue\n  if (alpha\nfunction theta eta test south\nlet north iota check\nlet south"
  },
  {
   "ok": true,
   "value": "function value build south\n}lambda green omega red\nexport check theta kappa west\n  if (epsilon north\n  if (ready south omega\n}ready build blue\n  return build theta\nfunction status record alpha omega\nfunction red done"
  },
  {
   "ok": true,
   "value": "export lambda beta zeta theta\n  return east green\n}lambda check done\nlet red\nlet red zeta east south\nexport alpha ready\nconst item theta theta delta\nexport red test value\n}entry item\nconst lambda red eta item\nexport zeta done omega blue\nexport south omega west delta\nfunction alpha test kappa"
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
const [a,b,c,d] = await Promise.all([
 tools.read({path:"metinike/qas1/laf/qas2/qine3.ts", offset:1, limit:200}),
 tools.read({path:"metinike/qas1/laf/qas2/qine4.ts", offset:1, limit:210}),
 tools.read({path:"metinike/qas1/nunu/qine5.ts", offset:1, limit:600}),
 tools.read({path:"metinike/qas1/laf/qas2/qine6.ts", offset:95, limit:20}),
]);
return [a,b,c,d].join("\n=====\n");
})(fake);
