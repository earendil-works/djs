// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "## Item build alpha theta beta check status lambda\n1. Check kappa south delta kappa entry north\n# Blue delta kappa\n1. Epsilon zeta alpha test done red theta test\n# Value test"
  },
  {
   "ok": true,
   "value": "function beta value gamma\nlet blue blue\nlet lambda iota\nconst delta item build zeta\nfunction east\nfunction test lambda red\n  if (alpha beta\nconst zeta theta blue\n}red lambda kappa blue\nexport epsilon\nlet status zeta\n  return ready\nfunction lambda\n}blue"
  },
  {
   "ok": true,
   "value": "  return north\nlet alpha epsilon status\nlet north\n  return gamma eta\n}beta west done done\nlet status delta delta item\nexport entry\nexport done delta omega\nfunction value ready ready\nfunction theta eta south\n  if (omega theta green entry\nfunction eta epsilon gamma blue\n}alpha test green status"
  },
  {
   "ok": true,
   "value": "1. Status build test blue record entry east alpha\n- East eta done gamma kappa alpha eta entry\nZeta zeta eta value omega epsilon record blue\n1. Lambda epsilon\n## Entry test\n# Beta record done record value\n1. Value lambda test test test\n## Eta iota blue done north entry beta\nEta zeta iota\n## Gamma lambda blue build north test\n## West delta east beta build west iota south"
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
const paths=["gopo/dir1/file2.md","gopo/dir1/src/dir3/file4.ts","gopo/dir1/src/dir3/file5.ts","gopo/dir1/README.md"];
await Promise.allSettled(paths.map(async path=>text({path,content:await tools.read({path})})));
})(fake);
