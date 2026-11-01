// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "{\n  \"items\": [\n    {\n      \"id\": 1,\n      \"name\": \"east\",\n      \"status\": \"ok\",\n      \"count\": 7,\n      \"tags\": [\n        \"red\",\n        \"kappa\"\n      ]\n    },\n    {\n      \"id\": 2,\n      \"name\": \"red\",\n      \"status\": \"closed\",\n      \"count\": 22,\n      \"tags\": [\n        \"kappa\",\n        \"check\"\n      ]\n    },\n    {\n      \"id\": 3,\n      \"name\": \"south\",\n      \"status\": \"closed\",\n      \"count\": 351,\n      \"tags\": [\n        \"alpha\"\n      ]\n    },\n    {\n      \"id\": 4,\n      \"name\": \"status\",\n      \"status\": \"ok\",\n      \"count\": 482,\n      \"tags\": [\n        \"test\"\n      ]\n    }\n  ],\n  \"total\": 4\n}"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"south\",\n    \"status\": \"open\",\n    \"count\": 82,\n    \"tags\": [\n      \"theta\",\n      \"red\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"item\",\n    \"status\": \"ok\",\n    \"count\": 93,\n    \"tags\": [\n      \"east\"\n    ]\n  },\n  {\n    \"id\": 3,\n    \"name\": \"value\",\n    \"status\": \"open\",\n    \"count\": 455,\n    \"tags\": [\n      \"value\",\n      \"value\",\n      \"south\"\n    ]\n  },\n  {\n    \"id\": 4,\n    \"name\": \"iota\",\n    \"status\": \"pending\",\n    \"count\": 73,\n    \"tags\": [\n      \"done\",\n      \"record\"\n    ]\n  }\n]"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"epsilon\",\n    \"status\": \"open\",\n    \"count\": 145,\n    \"tags\": [\n      \"ready\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"test\",\n    \"status\": \"ok\",\n    \"count\": 26,\n    \"tags\": [\n      \"epsilon\"\n    ]\n  },\n  {\n    \"id\": 3,\n    \"name\": \"kappa\",\n    \"status\": \"closed\",\n    \"count\": 3,\n    \"tags\": [\n      \"gamma\"\n    ]\n  },\n  {\n    \"id\": 4,\n    \"name\": \"done\",\n    \"status\": \"open\",\n    \"count\": 285,\n    \"tags\": [\n      \"kappa\"\n    ]\n  }\n]"
  },
  {
   "ok": true,
   "value": "{\n  \"items\": [\n    {\n      \"id\": 1,\n      \"name\": \"lambda\",\n      \"status\": \"open\",\n      \"count\": 39,\n      \"tags\": []\n    },\n    {\n      \"id\": 2,\n      \"name\": \"value\",\n      \"status\": \"pending\",\n      \"count\": 105,\n      \"tags\": [\n        \"ready\",\n        \"item\",\n        \"status\"\n      ]\n    },\n    {\n      \"id\": 3,\n      \"name\": \"zeta\",\n      \"status\": \"pending\",\n      \"count\": 169,\n      \"tags\": [\n        \"west\",\n        \"entry\"\n      ]\n    },\n    {\n      \"id\": 4,\n      \"name\": \"check\",\n      \"status\": \"open\",\n      \"count\": 213,\n      \"tags\": [\n        \"done\"\n      ]\n    }\n  ],\n  \"total\": 4\n}"
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
await Promise.allSettled([59875,141481,25860,106431].map(async n => text({number:n,...await tools.bash({command:`gh migim sega ${n} --quna qed1/qed2 --viti tasil,fatu,fepet,bitunepAt,bemira,medifiru,tuk`,timeout:30})})));
})(fake);
