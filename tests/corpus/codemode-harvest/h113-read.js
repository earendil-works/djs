// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "  return record iota iota east\nfunction item gamma red\n  return done west\nconst beta\n}entry\nexport red epsilon\nexport gamma status\n  if (build beta delta ready\nconst red delta gamma\nfunction done east\n  if (iota\nexport red status"
  },
  {
   "ok": true,
   "value": "- Entry delta delta green\n## Build item gamma omega\nAlpha north kappa\nDelta test kappa check green"
  },
  {
   "ok": true,
   "value": "const entry item build\nlet ready test record\n  if (kappa theta\n  return done status value"
  },
  {
   "ok": true,
   "value": "const delta check build eta\nlet green value zeta beta\n  return build red beta\nexport beta iota\n  return red kappa alpha west\nfunction epsilon red epsilon check\nfunction record epsilon alpha item\n  if (red"
  },
  {
   "ok": true,
   "value": "function gamma\nlet delta\nconst theta beta\n  if (red test alpha build\nexport beta\nlet test omega beta ready\nfunction delta south\nlet ready south green\nexport status ready red"
  },
  {
   "ok": true,
   "value": "}status\nfunction entry done eta\n  if (north south\nfunction status blue\nconst gamma\nlet theta north\nlet test"
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
for(const [path,offset,limit] of [["dir1/file2.py",475,495],["dir3/docs/file4.md",330,null],["dir3/src/dir5/file6.js",null,null],["dir3/src/dir7/file8.js",null,null],["dir3/src/core/file9.js",null,null],["dir3/src/dir10/file11.js",null,null]]) {text(path);text(await tools.read({path,offset,limit}));}
})(fake);
