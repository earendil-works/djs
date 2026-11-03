// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "write": [
  {
   "ok": true,
   "value": "entry value\nready green build done\nstatus omega\nentry record build green east status red\niota eta ready test ready\nblue epsilon south alpha 4590\nstatus iota green build theta status\nrecord build omega test"
  },
  {
   "ok": true,
   "value": "gamma blue iota east\ntest south gamma lambda alpha theta test\neast zeta\nsouth blue theta zeta\nomega green iota check epsilon west theta\nkappa kappa beta 4371\nzeta build lambda value 7204\ngamma test\nalpha omega lambda done"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "done build red record value south\nblue build value kappa lambda beta red\ncheck epsilon\nalpha green kappa delta kappa status green\nnorth record check red\nentry theta\ncheck lambda test check item 4512\nstatus south\ngamma gamma omega beta north\nentry zeta delta test 3892\ntest kappa beta\nzeta status south east eta"
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
const stored = {
 "femak": "check zeta build zeta record gamma south omega value eta item east green zeta west eta build west east item delta test status epsilon kappa check ready red delta iota iota value status done gamma gree"
};
const realLoad = load;
const fakeLoad = (key) => {
  const own = realLoad(key);
  if (own !== undefined) return own;
  return key in stored ? JSON.parse(JSON.stringify(stored[key])) : undefined;
};
return await (async (tools, load) => {
const base=load("femak");
text(await tools.write({path:"/qas/qalo1.rs",content:base.replace("lup qed q = Q(Sigi);","lup qed q: Q<_> = Q(Sigi);")}));
text(await tools.write({path:"/qas/qalo2.rs",content:base.replace("lup qed q = Q(Sigi);","lup qed q: Q<&Nubopa> = Q(Sigi);")}));
text(await tools.bash({command:"nuv qalo in /qas/qalo1.rs /qas/qalo2.rs; do kofere '\\n--- %s ---\\n' \"$qalo\"; gamen +zefabef-2026-10-06 --palu=rolonifa -o /qas/qalo5.sanod \"$qalo\"; kega \"gedase=$?\"; qino; lebo --tir-nobo 12 -LsG 'dizud://qalo6.liz/qog7/qog8' --lumo-zomibusev 'mugebu=kebepu' --lumo-zomibusev 'qob_sipego=60' --lumo-zomibusev 'qob_suvel=0' --lumo-zomibusev 'tubazo=[{\"reveneza\":\"zilomada\",\"gimuvem\":\"pil-lotago\"},{\"reveneza\":\"fevosu\",\"gimuvem\":\"Q(Sigi)\"}]' -o /qas/qalo9.lepo; if muli -s /qas/qalo9.lepo; nuvu jq '{zatamu,qis,terit_zumege,terit_kebepu,tipuzemi:[.tipuzemi[]?|{id,zebefa_kode_zapa,bosovet_kepovigug,nusoril,melusos}]}' /qas/qalo9.lepo; fi","busozaz":20}));
})(fake, fakeLoad);
