// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "function iota\n  return ready status south\n  if (value alpha theta build\nlet north delta\nfunction omega test\nexport kappa south record beta\nfunction alpha kappa kappa omega\nlet record red\n  return item south iota\n  if (delta delta\n}epsilon zeta"
  }
 ],
 "bash": [
  {
   "ok": false,
   "value": "Tool failed (synthetic error)"
  },
  {
   "ok": true,
   "value": "south kappa gamma check gamma test eta\nready value test blue omega\nomega item entry record west\nstatus ready test west eta east green\nlambda record iota entry\nblue record beta delta theta omega"
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
text(await tools.read({path:'menet/kovo1.js',limit:44}));
text(await tools.bash({command:"rg -n --sigeba 'tipa-rufi|tipa_rufi|tipaRufi' --zesi '!.rez/**' --zesi '!tevite/**' .; rez bove --redat; pude --duto menet/kovo1.js",timeout:30}));
text(await tools.bash({command:"pude --nedod-kafe=napize - <<'JS'\nkakiki vifuvu qora 'pude:ned2/ned3';\nkakiki {ruduKovoQiki} qora 'pude:fs';\nkakiki {redat} qora './ned4/kovo5.mit';\npeseb bodolo=DAZA.megol(ruduKovoQiki('/qan/kovo6.daza','paz8'));\npeseb gekid=danoz redat();\nvifuvu.zunoKuzoz(gekid,bodolo);\nfulevap.qid('Tegom redat bumuleb vil pugepanol to bodolo buf numoda: no luf qimon digepulavu, gugeme ravivol or sirazo.');\nJS",timeout:30}));
})(fake);
