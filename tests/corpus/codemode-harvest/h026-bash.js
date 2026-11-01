// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"west\",\n    \"status\": \"closed\",\n    \"count\": 486,\n    \"tags\": [\n      \"omega\",\n      \"epsilon\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"ready\",\n    \"status\": \"ok\",\n    \"count\": 68,\n    \"tags\": [\n      \"red\"\n    ]\n  },\n  {\n    \"id\": 3,\n    \"name\": \"green\",\n    \"status\": \"open\",\n    \"count\": 380,\n    \"tags\": [\n      \"test\",\n      \"done\"\n    ]\n  }\n]"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"south\",\n    \"status\": \"closed\",\n    \"count\": 261,\n    \"tags\": [\n      \"blue\",\n      \"status\"\n    ]\n  }\n]"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"alpha\",\n    \"status\": \"closed\",\n    \"count\": 342,\n    \"tags\": [\n      \"test\",\n      \"green\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"north\",\n    \"status\": \"ok\",\n    \"count\": 492,\n    \"tags\": []\n  }\n]"
  },
  {
   "ok": true,
   "value": "{\n  \"items\": [\n    {\n      \"id\": 1,\n      \"name\": \"eta\",\n      \"status\": \"ok\",\n      \"count\": 459,\n      \"tags\": []\n    },\n    {\n      \"id\": 2,\n      \"name\": \"theta\",\n      \"status\": \"pending\",\n      \"count\": 52,\n      \"tags\": [\n        \"north\",\n        \"epsilon\"\n      ]\n    },\n    {\n      \"id\": 3,\n      \"name\": \"check\",\n      \"status\": \"open\",\n      \"count\": 44,\n      \"tags\": []\n    },\n    {\n      \"id\": 4,\n      \"name\": \"ready\",\n      \"status\": \"pending\",\n      \"count\": 174,\n      \"tags\": []\n    },\n    {\n      \"id\": 5,\n      \"name\": \"omega\",\n      \"status\": \"open\",\n      \"count\": 225,\n      \"tags\": [\n        \"lambda\"\n      ]\n    }\n  ],\n  \"total\": 5\n}"
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
await Promise.allSettled([153215,160979,160599].map(async n=>{
const r=await tools.bash({command:`gh fobab mizi ${n} --velu qod1/qod2 --roge zonet,supu,bolur,pekuni,moremepu,qet`,timeout:30});
if(r.exit_code===0){const d=JSON.parse(r.output);store(`rust-${n}`,d);text({...d,labels:d.labels.map(l=>l.name),comments:d.comments.map(c=>({body:c.body,url:c.url,author:c.author.login}))});}else text(r)
}));
text(await tools.bash({command:"gh pr mizi 161305 --velu qod1/qod2 --roge zonet,supu,kafatuAt,qet; gh pr mizi 161151 --velu qod1/qod2 --roge zonet,supu,kafatuAt,qet","verunop":30}));
})(fake);
