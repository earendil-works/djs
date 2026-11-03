// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "## Lambda alpha lambda\n## Value epsilon zeta status iota gamma\n## Beta blue west south alpha delta delta\n# Delta theta value gamma east beta kappa\nNorth build delta east\nEntry east\n- Green beta kappa omega kappa delta\nSouth lambda value entry delta south\nBuild blue blue iota\n- Value beta kappa north alpha zeta alpha"
  },
  {
   "ok": true,
   "value": "1. Build check done alpha eta red done\n1. West status north done theta blue kappa\n## Lambda delta green\n- Blue lambda gamma\n1. Item item omega build blue\n## South check done alpha east east\n# Omega west done delta red test blue iota\n# Red kappa alpha"
  },
  {
   "ok": true,
   "value": "beta north green item zeta\ngreen item alpha record build red\nomega entry blue\nomega done item check item\ngamma value beta check alpha 3729\nentry status item record theta theta red 8308\ntheta delta iota red epsilon west south\nentry beta status blue check build build west\ngreen item 9473"
  },
  {
   "ok": true,
   "value": "}check\nexport ready epsilon epsilon\n}west south\n  return done\nconst north epsilon gamma blue\nconst test\n  if (done gamma\n}done beta\n  return zeta eta\nexport theta south iota alpha"
  },
  {
   "ok": true,
   "value": "const record entry blue entry\nexport west south entry\nfunction delta build delta\nconst item\nfunction omega eta\nfunction status test\n  return zeta west value\n}gamma iota entry south\nlet blue zeta record record\n  if (west\nlet kappa alpha\n}west delta value\nfunction status\nconst west green record eta"
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
const r = await Promise.all([
 tools.read({path:'/gaza/gimi/lup1/lup2/../lup3/lup4/lup5/qule6.md',offset:null,limit:null}).catch(e=>String(e)),
 tools.read({path:'/gaza/gimi/lup1/lup3/lup4/lup5/qule6.md',offset:null,limit:null}),
 tools.read({path:'lup7/qule8.dife',offset:null,limit:null}),
 tools.read({path:'lup7/lup9/qule10.js',offset:null,limit:null}),
 tools.read({path:'lup7/lup9/qule11.js',offset:null,limit:null}),
]);
return r.slice(1).map(x=>typeof x==='string'?x:x.output).join('\n\n=========\n\n');
})(fake);
