// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "}delta gamma done\nexport test north\nexport value epsilon theta value\n  return kappa\nexport eta\nconst entry check west\nconst alpha red kappa"
  },
  {
   "ok": true,
   "value": "export blue done gamma\n  return green\nexport epsilon epsilon\nconst west record\nexport omega eta lambda\n}north red theta\n  return south\nexport blue\nlet north\nfunction check\nexport west status\n  if (delta"
  },
  {
   "ok": true,
   "value": "}status\nfunction eta theta\nlet status\nexport eta eta done\nexport blue omega west blue\n  if (west omega"
  },
  {
   "ok": true,
   "value": "function east test omega theta\n  return green beta\nexport beta north epsilon\nexport lambda iota ready delta\nexport kappa red west\nfunction value check\nlet gamma omega\nfunction epsilon kappa build check\n  if (theta delta test north\nfunction blue check east value\n  return entry zeta\nfunction status omega eta\n  if (ready\nfunction omega iota eta"
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
const p="/home/user/dir1/dir2/packages/dir3/src/dir4/dir5/dir6/file7.ts";
for(const [offset,limit] of [[135,180],[315,129],[465,18],[586,22]]) text(await tools.read({path:p,offset,limit}));
})(fake);
