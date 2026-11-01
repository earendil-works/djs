// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"iota\",\n    \"status\": \"pending\",\n    \"count\": 172,\n    \"tags\": []\n  },\n  {\n    \"id\": 2,\n    \"name\": \"zeta\",\n    \"status\": \"pending\",\n    \"count\": 410,\n    \"tags\": [\n      \"value\",\n      \"eta\",\n      \"epsilon\"\n    ]\n  },\n  {\n    \"id\": 3,\n    \"name\": \"iota\",\n    \"status\": \"open\",\n    \"count\": 75,\n    \"tags\": [\n      \"done\",\n      \"record\"\n    ]\n  },\n  {\n    \"id\": 4,\n    \"name\": \"kappa\",\n    \"status\": \"pending\",\n    \"count\": 213,\n    \"tags\": [\n      \"green\"\n    ]\n  }\n]"
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
const issues = JSON.parse(await tools.read({path:"/qup/noto1.qude", offset:1, limit:100000}).then(s=>s));
return typeof issues;
})(fake);
