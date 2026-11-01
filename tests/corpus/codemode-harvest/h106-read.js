// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "export east red\n  if (item\n}lambda alpha"
  },
  {
   "ok": true,
   "value": "let done\n  return blue build\n}west delta alpha"
  },
  {
   "ok": true,
   "value": "let epsilon record\n  return zeta item\nexport south record kappa\n}alpha ready entry south"
  },
  {
   "ok": true,
   "value": "}item item eta\nconst delta theta gamma test\nconst record iota\nlet test\n}record lambda south entry\n  return iota\n}status build item\n  if (beta\n  if (blue eta done kappa\nconst east delta east\n}kappa south"
  },
  {
   "ok": true,
   "value": "  if (alpha epsilon east gamma\nexport build\n  if (north\n  if (test west"
  },
  {
   "ok": true,
   "value": "}east zeta\nexport north gamma epsilon\nexport record\nlet beta theta delta omega\n  return west zeta\n  return done gamma eta build\n  if (zeta\nconst south build\nfunction ready\nfunction eta beta\nlet east blue value west\n}value zeta\nfunction done check east"
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
const fs = ['src/dir1/file2.js','src/dir1/file3.js','src/dir4/file5.js','src/dir1/file6.js','src/dir1/file7.js','src/dir1/file8.js'];
const r = await Promise.all(fs.map(p=>tools.read({path:p})));
return r.map((t,i)=>`=== ${fs[i]}\n${t}`).join('\n');
})(fake);
