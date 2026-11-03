// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"done\",\n    \"status\": \"pending\",\n    \"count\": 217,\n    \"tags\": [\n      \"north\",\n      \"lambda\",\n      \"kappa\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"record\",\n    \"status\": \"open\",\n    \"count\": 230,\n    \"tags\": [\n      \"zeta\",\n      \"delta\"\n    ]\n  },\n  {\n    \"id\": 3,\n    \"name\": \"west\",\n    \"status\": \"pending\",\n    \"count\": 297,\n    \"tags\": [\n      \"check\",\n      \"south\",\n      \"zeta\"\n    ]\n  }\n]"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"record\",\n    \"status\": \"pending\",\n    \"count\": 39,\n    \"tags\": [\n      \"status\"\n    ]\n  }\n]"
  },
  {
   "ok": true,
   "value": "{\n  \"items\": [\n    {\n      \"id\": 1,\n      \"name\": \"check\",\n      \"status\": \"open\",\n      \"count\": 146,\n      \"tags\": [\n        \"kappa\",\n        \"eta\",\n        \"item\"\n      ]\n    },\n    {\n      \"id\": 2,\n      \"name\": \"build\",\n      \"status\": \"pending\",\n      \"count\": 359,\n      \"tags\": [\n        \"entry\",\n        \"lambda\"\n      ]\n    },\n    {\n      \"id\": 3,\n      \"name\": \"delta\",\n      \"status\": \"pending\",\n      \"count\": 386,\n      \"tags\": [\n        \"lambda\"\n      ]\n    }\n  ],\n  \"total\": 3\n}"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"done\",\n    \"status\": \"ok\",\n    \"count\": 384,\n    \"tags\": [\n      \"epsilon\",\n      \"done\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"east\",\n    \"status\": \"closed\",\n    \"count\": 126,\n    \"tags\": [\n      \"ready\",\n      \"green\",\n      \"zeta\"\n    ]\n  },\n  {\n    \"id\": 3,\n    \"name\": \"value\",\n    \"status\": \"closed\",\n    \"count\": 15,\n    \"tags\": [\n      \"green\",\n      \"build\",\n      \"check\"\n    ]\n  }\n]"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"omega\",\n    \"status\": \"pending\",\n    \"count\": 404,\n    \"tags\": []\n  },\n  {\n    \"id\": 2,\n    \"name\": \"entry\",\n    \"status\": \"ok\",\n    \"count\": 429,\n    \"tags\": []\n  },\n  {\n    \"id\": 3,\n    \"name\": \"north\",\n    \"status\": \"pending\",\n    \"count\": 310,\n    \"tags\": [\n      \"red\",\n      \"epsilon\"\n    ]\n  },\n  {\n    \"id\": 4,\n    \"name\": \"lambda\",\n    \"status\": \"closed\",\n    \"count\": 428,\n    \"tags\": []\n  }\n]"
  }
 ]
};
const fake = Object.create(null);
for (const name of Object.keys(results)) {
  const queue = results[name];
  fake[name] = (args) => {
    try {
      JSON.stringify(args);
    } catch (error) {
      return later(error, true);
    }
    const r = queue.length > 0 ? queue.shift() : { ok: true, value: "" };
    return r.ok ? later(r.value) : later(new Error(r.value), true);
  };
}
return await (async (tools) => {
const queries = ["label:I-unsound variance","label:I-unsound \"well-formed\"","label:I-unsound \"Drop\"","label:I-unsound \"bounds\"","\"covariant\" \"assignment\""];
await Promise.allSettled(queries.map(async q => text({query:q,...await tools.bash({command:`gh zumosa puganu --qete dat1/dat2 '${q}' --felap 100 --gana binogo,gabik,gamet,viz`,timeout:30})})));
})(fake);
