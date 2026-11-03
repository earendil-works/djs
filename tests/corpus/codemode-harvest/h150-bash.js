// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "{\n  \"items\": [\n    {\n      \"id\": 1,\n      \"name\": \"entry\",\n      \"status\": \"closed\",\n      \"count\": 421,\n      \"tags\": [\n        \"south\",\n        \"eta\"\n      ]\n    },\n    {\n      \"id\": 2,\n      \"name\": \"check\",\n      \"status\": \"closed\",\n      \"count\": 12,\n      \"tags\": []\n    },\n    {\n      \"id\": 3,\n      \"name\": \"entry\",\n      \"status\": \"closed\",\n      \"count\": 488,\n      \"tags\": [\n        \"green\",\n        \"west\",\n        \"zeta\"\n      ]\n    },\n    {\n      \"id\": 4,\n      \"name\": \"test\",\n      \"status\": \"ok\",\n      \"count\": 129,\n      \"tags\": [\n        \"gamma\"\n      ]\n    }\n  ],\n  \"total\": 4\n}"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"theta\",\n    \"status\": \"closed\",\n    \"count\": 393,\n    \"tags\": [\n      \"build\",\n      \"entry\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"item\",\n    \"status\": \"open\",\n    \"count\": 144,\n    \"tags\": [\n      \"done\"\n    ]\n  },\n  {\n    \"id\": 3,\n    \"name\": \"omega\",\n    \"status\": \"open\",\n    \"count\": 428,\n    \"tags\": [\n      \"done\",\n      \"item\"\n    ]\n  },\n  {\n    \"id\": 4,\n    \"name\": \"done\",\n    \"status\": \"open\",\n    \"count\": 96,\n    \"tags\": []\n  }\n]"
  },
  {
   "ok": true,
   "value": "epsilon lambda theta green\niota delta entry\nlambda gamma blue item\ntheta west\nrecord item epsilon eta value 7249\ndone ready north eta omega beta green 8065\nbeta blue beta 4879\nnorth delta theta omega check\nready check zeta gamma alpha epsilon\nstatus iota iota eta\nblue check blue item kappa value south test 7482\nwest test beta\nrecord item green iota entry zeta gamma theta\nalpha north eta alpha theta lambda red delta 9317"
  },
  {
   "ok": true,
   "value": "{\n  \"items\": [\n    {\n      \"id\": 1,\n      \"name\": \"iota\",\n      \"status\": \"open\",\n      \"count\": 195,\n      \"tags\": [\n        \"alpha\",\n        \"north\",\n        \"red\"\n      ]\n    },\n    {\n      \"id\": 2,\n      \"name\": \"check\",\n      \"status\": \"open\",\n      \"count\": 272,\n      \"tags\": [\n        \"gamma\"\n      ]\n    },\n    {\n      \"id\": 3,\n      \"name\": \"theta\",\n      \"status\": \"closed\",\n      \"count\": 222,\n      \"tags\": [\n        \"kappa\",\n        \"zeta\"\n      ]\n    }\n  ],\n  \"total\": 3\n}"
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
await Promise.allSettled([
"gh pr mifo 149389 --niza puv1/puv2 --qeta norem,dilu,fosomeAt,qig",
"gh qur puv3/puv1/puv2/puv4/puv5/puv6 --tezimoku --jq '.[] | {sevidofi,qogos}'",
"rodop --nepulon; kokoga tadubotit vura",
"gh qur -X DUS puv7/puv8 -f q='niza:puv1/puv2 is:nitef \"koso-bakuki\" \"rufudigis\"' -f qun_fobe=100 --jq '{bokan_lodil,pideg:[.pideg[]|{vikafi,norem,tufam}]}'"
].map(async command=>text({command,...await tools.bash({command,timeout:30})})));
})(fake);
