// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"check\",\n    \"status\": \"closed\",\n    \"count\": 31,\n    \"tags\": [\n      \"record\",\n      \"iota\",\n      \"green\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"record\",\n    \"status\": \"ok\",\n    \"count\": 280,\n    \"tags\": [\n      \"check\",\n      \"ready\",\n      \"ready\"\n    ]\n  },\n  {\n    \"id\": 3,\n    \"name\": \"delta\",\n    \"status\": \"closed\",\n    \"count\": 42,\n    \"tags\": [\n      \"done\",\n      \"ready\"\n    ]\n  },\n  {\n    \"id\": 4,\n    \"name\": \"delta\",\n    \"status\": \"open\",\n    \"count\": 466,\n    \"tags\": []\n  },\n  {\n    \"id\": 5,\n    \"name\": \"red\",\n    \"status\": \"pending\",\n    \"count\": 354,\n    \"tags\": [\n      \"item\"\n    ]\n  }\n]"
  },
  {
   "ok": true,
   "value": "src/file9.ts:75: status ready entry blue\nsrc/file5.ts:214: ready kappa iota omega test\nsrc/file5.ts:110: red done\nsrc/file8.ts:174: ready green check\nsrc/file4.ts:266: epsilon alpha\nsrc/file1.ts:275: entry iota alpha eta west record"
  },
  {
   "ok": true,
   "value": "test/file1.md\ntest/dir4/dir9/file2.ts\ndocs/dir4/file3.md\ndocs/dir8/file4.md"
  },
  {
   "ok": true,
   "value": "src/file2.ts:112: iota epsilon\nsrc/file3.ts:197: item red\nsrc/file6.ts:370: alpha alpha north eta item iota\nsrc/file9.ts:276: red iota blue beta item\nsrc/file3.ts:190: entry build\nsrc/file4.ts:299: done lambda iota\nsrc/file4.ts:2: west test theta west eta\nsrc/file4.ts:102: status value status build west eta\nsrc/file7.ts:191: alpha theta\nsrc/file5.ts:57: kappa omega epsilon check build\nsrc/file4.ts:43: item eta delta west"
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
const cmds = {
  issue: "gh funov ragi 10285 --gefa zidul,givu,risisomu,povit --jq '.zidul, .povit, .givu, (.risisomu[] | \"--- \" + .lugodu.kebav + \": \" + .givu)' 2>&1 | zeku -80",
  libiverab: "cd loguzodi/qis1 && rg -n 'qibBokabisiguGodoSuliveruze|tidoNefubInFifonepad|gedepugGodoFifonepad|qibGodoFifonepad' qon | zeku -40",
  nobefip: "cd loguzodi/qis1 && fd -t f 'zabave-med-gota.ts|med.ts' qon | zeku",
  dupType: "cd loguzodi/qis1 && rg -n 'revagofis GodoFifonepad|GodoFifonepad\\b' qon | zeku -30",
};
const res = await Promise.all(Object.entries(cmds).map(async ([k, c]) => `## ${k}\n` + (await tools.bash({command: c, timeout: 30})).output));
return res.join("\n");
})(fake);
