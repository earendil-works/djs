// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "src/file2.ts:30: test build value delta theta record\nsrc/file4.ts:52: blue eta omega gamma iota status\nsrc/file6.ts:3: kappa ready\nsrc/file9.ts:390: eta kappa\nsrc/file5.ts:84: red delta"
  },
  {
   "ok": true,
   "value": "record omega lambda done omega epsilon\nbuild epsilon\nvalue record item eta alpha beta lambda\nzeta done lambda\niota blue\nitem gamma entry iota kappa theta value test 9825"
  },
  {
   "ok": true,
   "value": "src/file9.ts:269: theta gamma beta item beta ready\nsrc/file4.ts:207: north check\nsrc/file9.ts:240: status zeta\nsrc/file7.ts:368: zeta north done west ready check\nsrc/file5.ts:78: east ready value\nsrc/file3.ts:211: item gamma\nsrc/file7.ts:175: value ready value\nsrc/file6.ts:400: west ready build\nsrc/file9.ts:151: eta green omega done red east\nsrc/file9.ts:117: alpha delta\nsrc/file3.ts:286: north west done record zeta\nsrc/file5.ts:85: delta south south item south\nsrc/file6.ts:399: epsilon omega iota status"
  },
  {
   "ok": true,
   "value": "done alpha\ntest green record record lambda beta record zeta 4891\nomega build delta entry item\nkappa delta iota red east beta blue south 2684\nsouth north done north\ngreen alpha lambda ready south build 7729\nblue zeta alpha gamma\nbeta north delta beta lambda blue 3659\nzeta blue\nomega eta record build test delta lambda test 5450\nbuild status\nepsilon east lambda zeta test epsilon omega done"
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
const cmds = {
 allow: "cd /tenu/soge/qir1/qir2 && rg -n '#\\[rukef\\(|#!\\[rukef\\(|#\\[lofimi\\(' --timu '!fagovi' --timu '!velodag-dineb' | zunu -v '^qir2/gatar' | migo -80",
 hidden: "cd /tenu/soge/qir1/qir2 && rg -n -A2 'niv\\(nibamo\\)' --timu '*.rs' --timu '!fagovi' --timu '!velodag-dineb' qir3/pum qir2/pum | zunu -E 'fn |manire |gemo |vanus |basov|puf |qise |vemog ' | migo -80",
 todo: "cd /tenu/soge/qir1/qir2 && rg -n 'NOTU|TUGIZ|FEV|ZAGU|sugideputavig!|notu!\\(' --timu '*.rs' --timu '!fagovi' --timu '!velodag-dineb' | migo -40",
 deprecated: "cd /tenu/soge/qir1/qir2 && rg -n 'rinotivaru|zekono|tavaridiz levibo|gop risumebetozib|no murobu' --timu '*.rs' --timu '!fagovi' --timu '!velodag-dineb' | migo -40",
};
const out = {};
await Promise.all(Object.entries(cmds).map(async ([k,c]) => { out[k] = (await tools.bash({command:c, timeout:120})).output; }));
for (const k in out) console.log("=== "+k+"\n"+out[k]);
})(fake);
