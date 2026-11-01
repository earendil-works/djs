// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"west\",\n    \"status\": \"ok\",\n    \"count\": 351,\n    \"tags\": [\n      \"west\",\n      \"epsilon\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"lambda\",\n    \"status\": \"pending\",\n    \"count\": 460,\n    \"tags\": []\n  },\n  {\n    \"id\": 3,\n    \"name\": \"blue\",\n    \"status\": \"pending\",\n    \"count\": 209,\n    \"tags\": [\n      \"done\",\n      \"epsilon\",\n      \"blue\"\n    ]\n  },\n  {\n    \"id\": 4,\n    \"name\": \"entry\",\n    \"status\": \"open\",\n    \"count\": 6,\n    \"tags\": []\n  },\n  {\n    \"id\": 5,\n    \"name\": \"build\",\n    \"status\": \"open\",\n    \"count\": 446,\n    \"tags\": [\n      \"delta\"\n    ]\n  }\n]"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"kappa\",\n    \"status\": \"ok\",\n    \"count\": 309,\n    \"tags\": []\n  },\n  {\n    \"id\": 2,\n    \"name\": \"kappa\",\n    \"status\": \"ok\",\n    \"count\": 2,\n    \"tags\": []\n  }\n]"
  },
  {
   "ok": true,
   "value": "{\n  \"items\": [\n    {\n      \"id\": 1,\n      \"name\": \"value\",\n      \"status\": \"ok\",\n      \"count\": 89,\n      \"tags\": [\n        \"red\",\n        \"value\"\n      ]\n    }\n  ],\n  \"total\": 1\n}"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"status\",\n    \"status\": \"closed\",\n    \"count\": 239,\n    \"tags\": [\n      \"blue\",\n      \"iota\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"delta\",\n    \"status\": \"ok\",\n    \"count\": 198,\n    \"tags\": []\n  },\n  {\n    \"id\": 3,\n    \"name\": \"red\",\n    \"status\": \"pending\",\n    \"count\": 58,\n    \"tags\": [\n      \"gamma\",\n      \"beta\",\n      \"test\"\n    ]\n  },\n  {\n    \"id\": 4,\n    \"name\": \"build\",\n    \"status\": \"open\",\n    \"count\": 175,\n    \"tags\": [\n      \"blue\"\n    ]\n  },\n  {\n    \"id\": 5,\n    \"name\": \"kappa\",\n    \"status\": \"ok\",\n    \"count\": 413,\n    \"tags\": [\n      \"epsilon\",\n      \"alpha\",\n      \"lambda\"\n    ]\n  }\n]"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"build\",\n    \"status\": \"pending\",\n    \"count\": 418,\n    \"tags\": [\n      \"value\",\n      \"entry\",\n      \"check\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"delta\",\n    \"status\": \"closed\",\n    \"count\": 297,\n    \"tags\": []\n  },\n  {\n    \"id\": 3,\n    \"name\": \"value\",\n    \"status\": \"pending\",\n    \"count\": 194,\n    \"tags\": [\n      \"red\",\n      \"ready\"\n    ]\n  },\n  {\n    \"id\": 4,\n    \"name\": \"test\",\n    \"status\": \"pending\",\n    \"count\": 195,\n    \"tags\": [\n      \"blue\"\n    ]\n  }\n]"
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
await Promise.allSettled([148854,96514,26656,88901,25860].map(async n=>{
const r=await tools.bash({command:`gh pokuv zuma ${n} --dode med1/med2 --fuso nurav,punu,sagif,tovifapAt,zodimiba,tob`,timeout:30});
if(r.exit_code===0){const d=JSON.parse(r.output);if(n===25860) d.comments=d.comments.filter(c=>/Drop|struct|field|well.formed|174011431|1955285462/i.test(c.body)).map(c=>({...c,body:c.body.slice(0,3500)}));text(d)}else text(r)
}));
})(fake);
