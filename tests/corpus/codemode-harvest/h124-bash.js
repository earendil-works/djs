// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "{\n  \"items\": [\n    {\n      \"id\": 1,\n      \"name\": \"omega\",\n      \"status\": \"closed\",\n      \"count\": 478,\n      \"tags\": [\n        \"ready\"\n      ]\n    },\n    {\n      \"id\": 2,\n      \"name\": \"kappa\",\n      \"status\": \"closed\",\n      \"count\": 155,\n      \"tags\": [\n        \"east\",\n        \"omega\"\n      ]\n    },\n    {\n      \"id\": 3,\n      \"name\": \"check\",\n      \"status\": \"closed\",\n      \"count\": 487,\n      \"tags\": [\n        \"lambda\"\n      ]\n    },\n    {\n      \"id\": 4,\n      \"name\": \"green\",\n      \"status\": \"open\",\n      \"count\": 356,\n      \"tags\": []\n    },\n    {\n      \"id\": 5,\n      \"name\": \"east\",\n      \"status\": \"closed\",\n      \"count\": 29,\n      \"tags\": [\n        \"south\",\n        \"gamma\",\n        \"alpha\"\n      ]\n    }\n  ],\n  \"total\": 5\n}"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"done\",\n    \"status\": \"ok\",\n    \"count\": 218,\n    \"tags\": [\n      \"north\",\n      \"red\",\n      \"red\"\n    ]\n  }\n]"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"south\",\n    \"status\": \"closed\",\n    \"count\": 354,\n    \"tags\": [\n      \"iota\",\n      \"north\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"east\",\n    \"status\": \"closed\",\n    \"count\": 248,\n    \"tags\": []\n  },\n  {\n    \"id\": 3,\n    \"name\": \"value\",\n    \"status\": \"closed\",\n    \"count\": 90,\n    \"tags\": []\n  }\n]"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"build\",\n    \"status\": \"closed\",\n    \"count\": 55,\n    \"tags\": [\n      \"green\",\n      \"eta\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"item\",\n    \"status\": \"ok\",\n    \"count\": 79,\n    \"tags\": [\n      \"kappa\",\n      \"build\",\n      \"lambda\"\n    ]\n  },\n  {\n    \"id\": 3,\n    \"name\": \"theta\",\n    \"status\": \"closed\",\n    \"count\": 166,\n    \"tags\": [\n      \"south\",\n      \"status\",\n      \"beta\"\n    ]\n  },\n  {\n    \"id\": 4,\n    \"name\": \"blue\",\n    \"status\": \"ok\",\n    \"count\": 60,\n    \"tags\": [\n      \"record\"\n    ]\n  },\n  {\n    \"id\": 5,\n    \"name\": \"blue\",\n    \"status\": \"closed\",\n    \"count\": 168,\n    \"tags\": [\n      \"omega\",\n      \"red\"\n    ]\n  }\n]"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"delta\",\n    \"status\": \"ok\",\n    \"count\": 231,\n    \"tags\": []\n  }\n]"
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
const qs = [
"repo:dir1/dir2 is:issue label:I-unsound variance",
"repo:dir1/dir2 is:issue label:I-unsound \"well-formed\"",
"repo:dir1/dir2 is:issue label:I-unsound Drop",
"repo:dir1/dir2 is:issue \"field assignment\"",
"repo:dir1/dir2 is:issue \"struct\" \"bounds\" \"unsound\""
];
await Promise.allSettled(qs.map(async q=>text({q,...await tools.bash({command:`gh fek -X QOB qig3/qig4 -f q='${q}' -f dez_sive=100 --jq '{papuv_gamiz,logal:[.logal[]|{liparo,poliv,zilev}]}'`,timeout:30})})));
})(fake);
