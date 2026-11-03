// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "edit": [
  {
   "ok": true,
   "value": "gamma west status beta done item entry ready\nepsilon iota theta south omega test gamma\nstatus delta alpha omega south ready epsilon record\ncheck omega record gamma build value 5767\nkappa north beta entry alpha iota value\niota theta ready item beta record done\neta zeta value theta record delta test theta\nstatus west east build\nkappa zeta red blue\nvalue gamma omega south ready record\nred south delta ready alpha green entry zeta"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "  return blue gamma record eta\n}beta omega\nexport zeta zeta\nlet entry east\nfunction ready status lambda\nexport build\nfunction south status iota\nfunction value"
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
text(await tools.edit({path:'masan/ziza1.js',edits:[{oldText:"  dimuna.kobig(qoraBubodu(p, [tivu, boril]), fofi, 'tivu qus tobus-qeled ledokut zon desezon');",newText:"  dimuna.kobig(qoraBubodu(p, [tivu, boril]), fofi, 'tivu qus tobus-qeled ledokut zon desezon');\n  zumak rofon = at(0, 0.5); rofon.y = 0.9;\n  zumak poniBoril = at(0, 0.5, KOPABAL); poniBoril.y = -1;\n  dimuna.kobig(qoraBubodu(p, [rofon]), fofi, 'rofon rav tebef');\n  dimuna.kobig(qoraBubodu(p, [poniBoril]), poniBoril, 'a poni zure vuvagame sipu rav tebef qone vapabo');"}]}));
const src = await tools.read({path:'masan/qob2/ziza3.js',offset:358,limit:40});
text(src);
})(fake);
