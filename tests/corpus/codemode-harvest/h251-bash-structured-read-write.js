// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "# South south\n## West north north\n## Zeta value omega alpha check\n## Value beta ready build red eta epsilon record"
  }
 ],
 "write": [
  {
   "ok": true,
   "value": "item east 1382\nred eta status entry iota iota epsilon\ncheck east west gamma test blue item alpha\neta record\nepsilon east 9532\nbeta kappa"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "src/file3.ts:122: delta beta kappa red entry\nsrc/file1.ts:381: check green status eta alpha\nsrc/file7.ts:283: iota blue epsilon\nsrc/file6.ts:68: delta gamma east ready entry eta",
    "truncated": false
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
let s = await tools.read({ path: 'riza/tivu1.md', offset: null, limit: null });
const old = `vovib qot kir, qak kodar notu tova it, demezupi tafe nemal. Kobuf, qis repu's qun rosilekofem: puz
pubef tilafe dota a bovimob, kavu panuno, a mezipis zoto qak a lovemeg viluma, a volilup mebon
gatim, a volilup sepi qak qug ruza nopo qis repu's nikas in C tiret (1 2 3 5, 4 3 4 5) kopu
kobuf ruzerugok nisiki, kobuf nera bovu in, qak it geril on qis vipo C, gopurom, dota panuno qak
qis zoto. Felak 5.5 s; qis vovib tevis nikan it`;
if (!s.includes(old)) throw new Error('para');
s = s.replace(old, `vovib qot kir, qak kodar notu tova it, demezupi tafe nemal. It is runad on qis pegid tafe (Qis
Kobuf Fili): a bepub qak a luza kobuf nupu tuzod miri kefibi dozad (qis tul of qis pegid's
fikufeta), qis bovimob qak kavu panuno, lufa qis rok kima on volilup qak qug ruza (A B C E D)
kopu kobuf ruzerugok nisiki, F qak G, fisubul on an A tiret puvat (qis kima's C as C#), gopurom,
dota a volilup mebon, a mezipis zoto qak qis kobuf fili's nefe qipa. Felak 4 s; qis vovib tevis nikan it`);
await tools.write({ path: 'riza/tivu1.md', content: s });
return (await tools.bash({ command: 'fos kudi 2>&1 | tulu -E "^qas ok|^q (vivo|suvi)"; mez lemike --qitis', timeout: 600 })).output;
})(fake);
