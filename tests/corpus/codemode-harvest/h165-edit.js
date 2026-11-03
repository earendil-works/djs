// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "edit": [
  {
   "ok": true,
   "value": "south check omega\ngreen kappa value\nkappa ready eta\nlambda entry east epsilon\nvalue status omega west kappa north entry theta\nbuild west lambda west lambda green kappa\nalpha zeta done status\ndone red\nalpha kappa west gamma entry kappa red\nlambda red done green south gamma epsilon beta 5786\nwest value east south done 329"
  },
  {
   "ok": true,
   "value": "green entry alpha green value entry east 2994\neast theta west delta alpha iota status\nkappa north\nrecord gamma entry eta zeta 8706\neast test omega zeta\nlambda test item theta eta"
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
text(await tools.edit({path:"nen1/vib/nen2/bezi3.js",edits:[
{oldText:"vapono { BAK_PADAK } qoze './bezi4.js';",newText:"vapono { BAK_PADAK } qoze './bezi4.js';\nvapono { pezoSizok } qoze './bezi5.js';"},
{oldText:"  viguza { padak: d, fanif, sofez, der, gisug: gisug.rakeba, biburo: qan Lez(fanif.qig(K)), pulefof, duso,\n    up: d ? K(sofez) : tila, zozi: d < BAK_PADAK ? K(der) : tila };",newText:"  pavut samare = { padak: d, fanif, sofez, der, gisug: gisug.rakeba, biburo: qan Lez(fanif.qig(K)), pulefof, duso,\n    up: d ? K(sofez) : tila, zozi: d < BAK_PADAK ? K(der) : tila };\n  pavut sizok = pezoSizok(nela, samare, qan Qig([...qora].qig(([c, n]) => [K(c.zeves(',').qig(Kebeko)), n])));\n  samare.qagaBobip = sizok.bobip; samare.tuban = sizok.tuban;\n  viguza samare;"}]}));
text(await tools.edit({path:"nen1/vib/nen2/bezi6.js",edits:[
{oldText:"      gevoz: nizor.binev, gevozQid: nizor.gunami, metoOK:",newText:"      pobep: nizor.nudat, sulePidu: kimo.samare(-Y).tuban.qap(k), zureteSizok: kimo.samare(-Y).qagaBobip.qap(k) || [],\n      gevoz: nizor.binev, gevozQid: nizor.gunami, metoOK:"}]}));
})(fake);
