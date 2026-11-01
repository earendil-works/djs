// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "function theta ready\n  return omega\nlet record\n  return eta green\nconst iota\nfunction omega east lambda gamma\n}build\nexport done"
  },
  {
   "ok": true,
   "value": "}lambda item\n  return omega\nlet west ready\nexport epsilon north green item\nfunction test item\n  if (build\nlet value delta build\nexport value entry"
  },
  {
   "ok": true,
   "value": "export check\n  return zeta build\nfunction check zeta eta ready\nfunction alpha red epsilon gamma\nlet theta alpha status theta\nlet entry\nfunction kappa beta delta build\n  if (beta blue south entry\n  if (south west\nlet test red\n  return lambda alpha\nlet status ready status test"
  },
  {
   "ok": true,
   "value": "  if (value green epsilon red\nexport green epsilon north\nfunction iota\nfunction done beta\nexport delta theta zeta\nexport south north\n  if (zeta build zeta status\nfunction item west delta build\nexport eta\nfunction delta blue\nexport north omega eta\n  if (alpha red"
  },
  {
   "ok": true,
   "value": "let blue record theta\nfunction gamma record west\nconst north north\n  if (build value eta theta\nlet blue green\n}record\nlet beta delta north south\nconst east\nlet epsilon\n  if (alpha item\nfunction entry gamma\nlet omega blue north west"
  },
  {
   "ok": true,
   "value": "function epsilon status\n  return epsilon epsilon item north\n  if (omega check\nlet check omega green south\n  return value\nconst test\nconst alpha epsilon green status\nlet done\nexport theta zeta\nconst east test kappa"
  },
  {
   "ok": true,
   "value": "# Eta beta beta ready red\n# Done iota south record test blue build\n## Record theta\n# Gamma ready item red epsilon green\n1. Gamma iota\n1. Eta green\nTheta theta omega eta alpha zeta value done\n## Item iota iota ready omega\n1. Value delta\nEntry lambda eta gamma\nTheta ready blue lambda\n1. Status iota theta east west test eta"
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
const paths = ['dir1/src/dir2/file3.js','dir1/src/dir2/file4.js','dir1/src/dir5/file6.js','dir1/tests/file7.js','dir1/tests/file8.js','dir1/src/dir9/file10.js','dir1/docs/file11.md'];
await Promise.allSettled(paths.map(async path => text({path, content:await tools.read({path, limit:path.endsWith('fudituv-lovuga.js') ? 160 : undefined})})));
})(fake);
