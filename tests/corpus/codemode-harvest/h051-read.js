// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "1. Check green done done east kappa item\n- Ready blue iota lambda\n- West omega\n## Kappa west gamma green lambda west value check\n- Alpha record lambda delta epsilon"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"test\",\n    \"status\": \"open\",\n    \"count\": 7,\n    \"tags\": [\n      \"delta\",\n      \"lambda\",\n      \"kappa\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"item\",\n    \"status\": \"pending\",\n    \"count\": 306,\n    \"tags\": [\n      \"alpha\",\n      \"check\"\n    ]\n  },\n  {\n    \"id\": 3,\n    \"name\": \"beta\",\n    \"status\": \"ok\",\n    \"count\": 224,\n    \"tags\": []\n  },\n  {\n    \"id\": 4,\n    \"name\": \"kappa\",\n    \"status\": \"pending\",\n    \"count\": 402,\n    \"tags\": [\n      \"status\",\n      \"build\",\n      \"alpha\"\n    ]\n  }\n]"
  },
  {
   "ok": true,
   "value": "- Gamma kappa delta zeta record green omega south\n- Beta epsilon eta kappa east zeta delta\nGamma build delta south west build done zeta\nTheta check item done ready west"
  },
  {
   "ok": true,
   "value": "eta south green build east gamma entry delta 6187\nstatus theta north green red blue\nblue omega omega item item blue lambda 1523\neta iota value beta test lambda eta\nblue north zeta green 2760\nstatus record delta ready east ready green 6771\nepsilon entry gamma\nnorth done blue west delta 2346\neta north check 3596"
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
const files = ["README.md", "package.json", "CONTRIBUTING.md", "host1.example.kir"];
const results = await Promise.allSettled(files.map(p => tools.read({ path: p, offset: null, limit: 40 })));
results.forEach((r, i) => {
  const body = r.status === "fulfilled" ? (typeof r.value === "string" ? r.value : "[image]") : `ERROR: ${r.reason}`;
  text(`=== ${files[i]} ===\n${body.slice(0, 1500)}\n`);
});
})(fake);
