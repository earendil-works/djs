// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": "status entry alpha beta theta 9949\nwest epsilon\nkappa red item record lambda lambda west omega\nentry west north entry gamma 8935\nred west red beta delta east value 7206"
  },
  {
   "ok": true,
   "value": "build blue delta record\nsouth record record test\nstatus blue green alpha\ngamma zeta test entry ready alpha\nstatus eta\nblue east\nred record test east 9956\neta epsilon kappa theta build ready\niota status eta beta omega item north delta\nred north gamma zeta\ndone ready status\ngreen ready record build\ngreen item beta zeta record alpha\ndelta status south build south alpha"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "## East iota entry test theta south item\n## Epsilon ready\n## Blue kappa red status eta delta ready\nSouth entry delta done\n1. Record east"
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
await Promise.allSettled([
tools.bash({command:"gofi -Ls 'senab://qani2.qad/qani3.puf' -o /sak/qani4.puf; gh sid pim5/pim6/pim7/pim8/pim9/pim10/qen/pim11/qes.rs -H 'Lagefe: pim12/qani13.zaz+dobo' > /sak/qani14.rs; rg -n 'ZeraGanolo|zera.ganolo|sapak_mever|sapak_rozud|taref_mever_velu|Kadi|kikib' /sak/qani14.rs | gofi -65","lidagig":30}),
tools.bash({command:"badud +zofunol-2026-10-06 /sak/qani15.rs -Nofer-zum=vuv -Nofer-zum-pim=/sak/pim16 --fuge=loreroze -o /sak/qani17.giruv; ls /sak/pim16 | gofi -12","lidagig":30})
].map(async p=>text(await p)));
text(await tools.read({path:"/sak/qani4.puf",offset:null,limit:null}));
})(fake);
