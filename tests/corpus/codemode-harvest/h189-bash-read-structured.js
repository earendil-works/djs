// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "export iota\n  return eta\n  return alpha iota\n  if (zeta red north\nexport kappa iota theta\n  return entry green east\nfunction status kappa\nfunction alpha north\nfunction beta build epsilon\n  return green lambda status"
  },
  {
   "ok": true,
   "value": "}south\nexport south entry\nconst kappa status blue eta\nexport lambda done\n}iota ready\n}record south omega iota"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "src/file9.ts:340: omega red alpha record\nsrc/file6.ts:218: blue record entry build lambda\nsrc/file1.ts:358: status blue south beta build green\nsrc/file9.ts:193: east kappa red build epsilon west\nsrc/file4.ts:221: west test lambda\nsrc/file5.ts:397: iota gamma beta eta\nsrc/file2.ts:190: east omega ready done\nsrc/file7.ts:231: north theta build\nsrc/file8.ts:129: omega check alpha zeta iota\nsrc/file4.ts:300: epsilon west ready\nsrc/file3.ts:250: iota ready epsilon\nsrc/file8.ts:279: build lambda test zeta south\nsrc/file4.ts:289: south north south entry"
  }
 ],
 "qut__blender__execute_blender_code": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "type": "text",
      "text": "(structured)"
     }
    ],
    "structuredContent": {
     "epsilon": {
      "value": "theta green"
     },
     "east": {
      "value": "east west east"
     }
    }
   }
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
const rs=await Promise.allSettled([
tools.read({path:"dof1/rimi2.py",offset:null,limit:null}),
tools.read({path:"dof1/rimi3.py",offset:1,limit:650}),
tools.bash({command:"rg -n '^niz |^gukap |zivipi_|qaba|QIM' dof1/rimi3.py; mez vunati --fokog",timeout:10}),
tools.qut__blender__execute_blender_code({code:"givoge qat\ndamovu={'tuzubaz':soba1.tarodef.qab.tuzubaz_dororu,'rimi':qat.nata.ligasoki,'netav':qat.semanes.netav.zuze}"})
]);rs.forEach((r,i)=>text({i,...r}));
})(fake);
