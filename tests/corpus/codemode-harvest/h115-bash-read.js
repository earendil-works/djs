// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "  if (north east entry\n}entry status north\nfunction beta blue omega\n  return check value record\n  return theta north\nconst check west item\n  return build epsilon epsilon"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "src/file3.ts:244: north blue east status done\nsrc/file2.ts:81: epsilon eta eta delta west\nsrc/file9.ts:172: entry omega\nsrc/file1.ts:349: omega west epsilon epsilon\nsrc/file4.ts:186: record entry epsilon\nsrc/file1.ts:246: value check ready item\nsrc/file2.ts:87: zeta iota lambda\nsrc/file5.ts:31: done theta south\nsrc/file7.ts:223: kappa delta zeta omega status\nsrc/file5.ts:134: red done record\nsrc/file9.ts:302: test alpha record east blue\nsrc/file9.ts:310: green beta\nsrc/file4.ts:162: east build epsilon\nsrc/file8.ts:245: east record"
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
const base="/home/user/dir1/dir2/dir3/";
const r = await Promise.all([
 tools.read({path:base+"ruv4/ruv5/ruv6/moso7.dagim",offset:1,limit:405}),
 tools.bash({command:`cd ${base}ruv4/ruv5/ruv6 && zava -n 'makemu \\|gafe \\|quz simu' FarumigVofe.dagim | taza -60`}),
]);
return r.map(x=>typeof x==="string"?x:x.output).join("\n=====\n");
})(fake);
