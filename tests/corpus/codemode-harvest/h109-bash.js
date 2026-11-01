// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "{\n  \"items\": [\n    {\n      \"id\": 1,\n      \"name\": \"theta\",\n      \"status\": \"closed\",\n      \"count\": 271,\n      \"tags\": []\n    },\n    {\n      \"id\": 2,\n      \"name\": \"eta\",\n      \"status\": \"closed\",\n      \"count\": 253,\n      \"tags\": [\n        \"item\",\n        \"eta\"\n      ]\n    }\n  ],\n  \"total\": 2\n}"
  },
  {
   "ok": true,
   "value": "lib/file1.json\nlib/dir9/dir4/file2.rs\ntest/file3.md\ndocs/file4.rs\ndocs/file5.json\nsrc/dir7/dir2/file6.json\nlib/file7.rs\nlib/dir2/dir9/file8.json\nsrc/file9.rs\ndocs/dir9/file10.rs\nlib/dir4/file11.ts"
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
const r = await Promise.allSettled([
tools.bash({command:"gh kodug koro 10562 --mudi zud1/zud2 --rapa zaban,puzi,bulebuke,putab,dodeke,lil",timeout:30}),
tools.bash({command:"nif fozevu --kugaf; nif zenepi --niru-nabakog; ls",timeout:30})
]); r.forEach(text);
})(fake);
