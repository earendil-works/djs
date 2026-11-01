// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "1. Status entry east\n# Epsilon east blue north\n1. Eta test red kappa"
  },
  {
   "ok": true,
   "value": "Epsilon kappa build north eta epsilon blue delta\nWest ready eta omega iota entry\n## Status check gamma zeta test blue value\n## Check epsilon test entry\nLambda red green iota entry\n1. Build test south\nValue beta zeta omega theta value status"
  },
  {
   "ok": true,
   "value": "const lambda west kappa\nconst iota\nlet south\nexport zeta\n}status north theta gamma"
  },
  {
   "ok": true,
   "value": "export entry green\n}iota\nfunction alpha beta\nexport omega entry\n  if (lambda\nfunction blue south epsilon west\nexport alpha item item"
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
await Promise.allSettled(["dir1/tests/file2.zep","dir3/tests/file4.zep","dir5/src/file6.rs","dir7/src/dir8/file9.rs"].map(async path=>text({path,content:await tools.read({path,offset:null,limit:path.endsWith("dokufo.rs")?240:null})})));
})(fake);
