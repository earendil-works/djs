// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "}west\n}ready red\nfunction value\n  if (west west\nlet lambda gamma\nexport delta blue build\nfunction check red ready\n}value east south done"
  },
  {
   "ok": true,
   "value": "let theta\n}blue zeta test\n}done blue\nexport alpha\nconst ready value theta\nconst west kappa\n  return lambda red\nfunction beta ready item beta\n  if (entry\nfunction ready green check ready\n  return blue north value\n  if (west lambda"
  },
  {
   "ok": true,
   "value": "}red eta\nlet item north\n}alpha value\nfunction alpha eta gamma\n  if (entry build north lambda\nexport lambda build south build"
  },
  {
   "ok": true,
   "value": "  if (lambda test omega epsilon\nconst zeta gamma\n  return epsilon eta red ready\n  if (done epsilon east record\nlet epsilon gamma green\n}item eta\n}gamma\nfunction test zeta\n}kappa\nexport eta test check build\nconst item value\n  return status green epsilon blue\n  return omega east"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "src/file1.ts:12: blue done gamma\nsrc/file8.ts:56: west value\nsrc/file3.ts:44: iota item status\nsrc/file7.ts:289: zeta kappa kappa test status lambda\nsrc/file1.ts:130: epsilon west\nsrc/file5.ts:102: done status omega\nsrc/file4.ts:298: done gamma item blue item gamma\nsrc/file1.ts:31: entry omega kappa alpha\nsrc/file6.ts:288: north north theta\nsrc/file3.ts:20: green record item check theta"
  }
 ],
 "zun__blender__execute_blender_code": [
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
     "kappa": {
      "value": "delta status"
     },
     "test": {
      "value": "gamma value"
     }
    }
   }
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
const results = await Promise.allSettled([
tools.read({path:"qat1/qise2.py",offset:null,limit:null}),
tools.read({path:"qat1/qise3.py",offset:1,limit:120}),
tools.read({path:"qat4/piv/qat5/qise6.js",offset:null,limit:null}),
tools.read({path:"qat4/piv/qat5/qise7.js",offset:null,limit:null}),
tools.bash({command:"zebi -90 qat1/qise3.py; rg -n 'seforu_zeruk|seforu_|zukodopa|rer fove' qat1/qise8.py",timeout:10}),
tools.zun__blender__execute_blender_code({code:"fadivi zet\ndozuna = {'tudiler': kesu1.nilomup.qop.tudiler_maroko, 'qise': zet.fola.bimetabe, 'tenakiz': mip(zet.fola.tenakiz)}"})
]); results.forEach((r,i)=>text({i,...r}));
})(fake);
