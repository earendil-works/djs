// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "{\n  \"items\": [\n    {\n      \"id\": 1,\n      \"name\": \"done\",\n      \"status\": \"closed\",\n      \"count\": 301,\n      \"tags\": []\n    },\n    {\n      \"id\": 2,\n      \"name\": \"zeta\",\n      \"status\": \"open\",\n      \"count\": 125,\n      \"tags\": [\n        \"entry\"\n      ]\n    },\n    {\n      \"id\": 3,\n      \"name\": \"beta\",\n      \"status\": \"open\",\n      \"count\": 133,\n      \"tags\": []\n    },\n    {\n      \"id\": 4,\n      \"name\": \"ready\",\n      \"status\": \"closed\",\n      \"count\": 356,\n      \"tags\": []\n    }\n  ],\n  \"total\": 4\n}"
  },
  {
   "ok": true,
   "value": "{\n  \"items\": [\n    {\n      \"id\": 1,\n      \"name\": \"delta\",\n      \"status\": \"pending\",\n      \"count\": 432,\n      \"tags\": [\n        \"south\",\n        \"north\"\n      ]\n    },\n    {\n      \"id\": 2,\n      \"name\": \"done\",\n      \"status\": \"closed\",\n      \"count\": 36,\n      \"tags\": [\n        \"done\",\n        \"east\",\n        \"test\"\n      ]\n    },\n    {\n      \"id\": 3,\n      \"name\": \"blue\",\n      \"status\": \"closed\",\n      \"count\": 136,\n      \"tags\": [\n        \"value\",\n        \"value\",\n        \"value\"\n      ]\n    },\n    {\n      \"id\": 4,\n      \"name\": \"delta\",\n      \"status\": \"open\",\n      \"count\": 496,\n      \"tags\": []\n    },\n    {\n      \"id\": 5,\n      \"name\": \"status\",\n      \"status\": \"pending\",\n      \"count\": 71,\n      \"tags\": [\n        \"value\",\n        \"theta\"\n      ]\n    }\n  ],\n  \"total\": 5\n}"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"build\",\n    \"status\": \"ok\",\n    \"count\": 476,\n    \"tags\": [\n      \"item\",\n      \"east\",\n      \"zeta\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"item\",\n    \"status\": \"open\",\n    \"count\": 408,\n    \"tags\": [\n      \"iota\"\n    ]\n  },\n  {\n    \"id\": 3,\n    \"name\": \"status\",\n    \"status\": \"ok\",\n    \"count\": 500,\n    \"tags\": [\n      \"north\",\n      \"delta\"\n    ]\n  },\n  {\n    \"id\": 4,\n    \"name\": \"iota\",\n    \"status\": \"ok\",\n    \"count\": 378,\n    \"tags\": [\n      \"delta\",\n      \"east\",\n      \"zeta\"\n    ]\n  }\n]"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"iota\",\n    \"status\": \"ok\",\n    \"count\": 41,\n    \"tags\": [\n      \"blue\",\n      \"record\",\n      \"record\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"west\",\n    \"status\": \"open\",\n    \"count\": 151,\n    \"tags\": [\n      \"check\"\n    ]\n  },\n  {\n    \"id\": 3,\n    \"name\": \"gamma\",\n    \"status\": \"ok\",\n    \"count\": 420,\n    \"tags\": [\n      \"build\",\n      \"check\",\n      \"zeta\"\n    ]\n  }\n]"
  },
  {
   "ok": true,
   "value": "{\n  \"items\": [\n    {\n      \"id\": 1,\n      \"name\": \"gamma\",\n      \"status\": \"pending\",\n      \"count\": 249,\n      \"tags\": []\n    },\n    {\n      \"id\": 2,\n      \"name\": \"record\",\n      \"status\": \"pending\",\n      \"count\": 313,\n      \"tags\": []\n    },\n    {\n      \"id\": 3,\n      \"name\": \"epsilon\",\n      \"status\": \"open\",\n      \"count\": 140,\n      \"tags\": [\n        \"check\"\n      ]\n    },\n    {\n      \"id\": 4,\n      \"name\": \"record\",\n      \"status\": \"ok\",\n      \"count\": 160,\n      \"tags\": []\n    }\n  ],\n  \"total\": 4\n}"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"ready\",\n    \"status\": \"ok\",\n    \"count\": 57,\n    \"tags\": []\n  },\n  {\n    \"id\": 2,\n    \"name\": \"green\",\n    \"status\": \"pending\",\n    \"count\": 259,\n    \"tags\": [\n      \"alpha\"\n    ]\n  }\n]"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"delta\",\n    \"status\": \"open\",\n    \"count\": 139,\n    \"tags\": []\n  }\n]"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"check\",\n    \"status\": \"pending\",\n    \"count\": 46,\n    \"tags\": []\n  },\n  {\n    \"id\": 2,\n    \"name\": \"item\",\n    \"status\": \"ok\",\n    \"count\": 243,\n    \"tags\": [\n      \"red\",\n      \"zeta\",\n      \"north\"\n    ]\n  }\n]"
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
const issues=[["rust",161038],["rust",161471],["rust",163886],["rust",160669],["rust",160670],["rust",53142],["tupopuma",166],["tupopuma",172]];
await Promise.allSettled(issues.map(async ([repo,n])=>{
const r=await tools.bash({command:`gh fikim biza ${n} --rubu rofi-kufe/${repo} --qagu gidot,qina,gadod,masise,zotunibo,qap`,timeout:30});
if(r.exit_code===0) {
 const d=JSON.parse(r.output);
 store(`${repo}-${n}`,d);
 text({...d,labels:d.labels.map(l=>l.name),comments:d.comments.map(c=>({body:c.body,url:c.url,author:c.author.login}))});
}else text(r);
}));
})(fake);
