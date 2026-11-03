// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"blue\",\n    \"status\": \"open\",\n    \"count\": 106,\n    \"tags\": [\n      \"check\",\n      \"south\",\n      \"test\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"alpha\",\n    \"status\": \"open\",\n    \"count\": 200,\n    \"tags\": [\n      \"ready\",\n      \"alpha\"\n    ]\n  }\n]"
  },
  {
   "ok": true,
   "value": "[\n  {\n    \"id\": 1,\n    \"name\": \"status\",\n    \"status\": \"closed\",\n    \"count\": 326,\n    \"tags\": [\n      \"south\",\n      \"epsilon\"\n    ]\n  },\n  {\n    \"id\": 2,\n    \"name\": \"eta\",\n    \"status\": \"open\",\n    \"count\": 155,\n    \"tags\": []\n  },\n  {\n    \"id\": 3,\n    \"name\": \"kappa\",\n    \"status\": \"pending\",\n    \"count\": 130,\n    \"tags\": [\n      \"ready\",\n      \"item\",\n      \"alpha\"\n    ]\n  }\n]"
  },
  {
   "ok": true,
   "value": "const record eta north north\n  return theta test\n}east ready red status"
  },
  {
   "ok": true,
   "value": "check north beta lambda east test 3635\nbuild eta zeta green kappa north zeta 9972\nblue omega iota blue theta lambda status north\nvalue done build epsilon west lambda west east"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "record north iota west build green 7248\nbeta delta value status alpha\ngreen omega ready check entry omega\nblue status\nnorth north red\nzeta item 1788\nready alpha south kappa check beta done\nblue value delta value west zeta item 3317\nready done record record delta record alpha omega\neast red check east"
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
for(const path of ["/tmp/file1.json","/tmp/file2.json","scripts/file3.mjs","dir4/dir5/file6.yml"]) text({path,content:await tools.read({path,offset:1,limit:2000})});
text(await tools.bash({command:"bip qakir --regeba futotopo/teb7/teb8 --qede=kad --qope > /qon/suke9.qope; potolo 'qakir kome: %s\\pedalif: /qon/suke9.qope\\n' \"$?\"; fuv qame -- sidusab-doku.qope","peredis":120}));
})(fake);
