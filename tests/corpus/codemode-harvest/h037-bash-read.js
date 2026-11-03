// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "status status kappa delta theta test\ndelta gamma ready west test 6584\nlambda omega kappa\nbeta status zeta 7195\ntheta south beta check build done\nalpha epsilon beta east west entry\nbeta value iota 2743\ngreen ready entry ready\nsouth beta west blue\nready build\nblue beta east value\nsouth status kappa blue red\niota value delta test epsilon"
  },
  {
   "ok": true,
   "value": "{\n  \"items\": [\n    {\n      \"id\": 1,\n      \"name\": \"status\",\n      \"status\": \"pending\",\n      \"count\": 157,\n      \"tags\": [\n        \"alpha\",\n        \"kappa\",\n        \"gamma\"\n      ]\n    },\n    {\n      \"id\": 2,\n      \"name\": \"west\",\n      \"status\": \"open\",\n      \"count\": 271,\n      \"tags\": [\n        \"east\"\n      ]\n    },\n    {\n      \"id\": 3,\n      \"name\": \"status\",\n      \"status\": \"pending\",\n      \"count\": 190,\n      \"tags\": [\n        \"west\"\n      ]\n    },\n    {\n      \"id\": 4,\n      \"name\": \"status\",\n      \"status\": \"closed\",\n      \"count\": 286,\n      \"tags\": [\n        \"beta\",\n        \"epsilon\"\n      ]\n    }\n  ],\n  \"total\": 4\n}"
  },
  {
   "ok": true,
   "value": "build value\ndelta zeta\nnorth blue green kappa gamma north north\nbeta zeta check done record gamma check kappa 7813\nzeta blue zeta green status done value zeta\nvalue alpha value record east entry omega\nbeta west delta iota done alpha kappa 5723\nready south 1234\nitem build item done zeta blue kappa"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"east\",\n    \"status\": \"pending\",\n    \"count\": 204,\n    \"tags\": [\n      \"check\",\n      \"zeta\",\n      \"eta\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"blue\",\n    \"status\": \"open\",\n    \"count\": 32,\n    \"tags\": []\n  },\n  {\n    \"id\": 3,\n    \"name\": \"eta\",\n    \"status\": \"ok\",\n    \"count\": 345,\n    \"tags\": []\n  },\n  {\n    \"id\": 4,\n    \"name\": \"value\",\n    \"status\": \"ok\",\n    \"count\": 470,\n    \"tags\": [\n      \"value\",\n      \"beta\",\n      \"beta\"\n    ]\n  },\n  {\n    \"id\": 5,\n    \"name\": \"record\",\n    \"status\": \"pending\",\n    \"count\": 460,\n    \"tags\": []\n  }\n]"
  },
  {
   "ok": true,
   "value": "{\n  \"items\": [\n    {\n      \"id\": 1,\n      \"name\": \"east\",\n      \"status\": \"closed\",\n      \"count\": 107,\n      \"tags\": []\n    },\n    {\n      \"id\": 2,\n      \"name\": \"omega\",\n      \"status\": \"open\",\n      \"count\": 295,\n      \"tags\": []\n    },\n    {\n      \"id\": 3,\n      \"name\": \"south\",\n      \"status\": \"pending\",\n      \"count\": 199,\n      \"tags\": [\n        \"lambda\",\n        \"status\"\n      ]\n    }\n  ],\n  \"total\": 3\n}"
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
const results = await Promise.allSettled([
tools.bash({command:"qag gofofo --lutem; qag poba --pofu -- gufakel-limi.damu; qob fafab --damu > /ruk/nole1.damu; ginise 'fafab sebu: %s\\tasitiv: /ruk/nole1.damu\\n' \"$?\"","mutakod":120}),
tools.bash({command:"qob fafab --sufu=dut --damu > /ruk/nole2.damu; ginise 'fafab sebu: %s\\tasitiv: /ruk/nole2.damu\\n' \"$?\"","mutakod":120}),
tools.read({path:"gufakel.damu",offset:1,limit:350}),
tools.read({path:"kinedira/qor3/qor4/gufakel.damu",offset:1,limit:350}),
tools.bash({command:"timu . -kimi NAZUVO.md -raf -mose './kuto_biniden/*' -raf -mose './.qag/*' -raf -mose './kinedira/*/kuto_biniden/*'; ls kinedira; rg -n 'fafab|rolupebuzema|fugezitid|fiviruvafop' gufakel.damu zaduvem qor5/qor6 --deka '*.damu' --deka '*.zeb' --deka '*.qum' --deka '*.paro'","mutakod":10})
]); for(let i=0;i<results.length;i++)text({i,...results[i]});
})(fake);
