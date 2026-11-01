// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "edit": [
  {
   "ok": true,
   "value": "south omega alpha record item eta green east\nnorth delta gamma west red kappa omega\ngreen eta\nzeta omega alpha\nblue ready blue value delta north ready\ndone north gamma theta iota alpha entry kappa\ndone west beta blue\nkappa eta west"
  },
  {
   "ok": true,
   "value": "east theta status blue\ngamma omega north south north iota record\nalpha ready\ndelta green\ngreen kappa delta alpha\nstatus kappa red\nomega alpha\nepsilon theta epsilon alpha red record eta 7043\nlambda west test north iota zeta epsilon 1007\nbuild item zeta test epsilon north\ndone green beta epsilon red 8772\nred item done alpha north green gamma entry\nrecord status done theta ready omega"
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
const r = [];
r.push(await tools.edit({path: 'vob/pov1/tadi2.js', edits: [
 {oldText: "difum ril ruzid qam nanov luz vaze: luz bigu gebo qak it fupu\n// luz memub, pov1/tadi3.js famube.qonOK). Qam pezep a topodat.", newText: "difum ril ruzid qam nanov luz vaze: luz bigu gebo qak it fupu\n// luz memub, as qig bigu deve: pov1/tadi3.js). Qam pezep a topodat."},
 {oldText: "  if (w.i >= zeno.kufoze) { w.vebe = qika; p.qonOK = dufol; sozosa qil(w.x, w.z, qika); }\n  w.vebe = dufol;\n  p.qonOK = zeno[w.i].kanem === MEMUB;   // (fupu luz memub: qak luz buvo, pov1/tadi3.js)\n", newText: "  if (w.i >= zeno.kufoze) { w.vebe = qika; sozosa qil(w.x, w.z, qika); }\n  w.vebe = dufol;\n"}]}));
r.push(await tools.edit({path: 'vob/pov4/tadi5.js', edits: [
 {oldText: "tuzudi lagegefa dupevModem() { famube.qonOK = dufol; modem.manu", newText: "tuzudi lagegefa dupevModem() { modem.manu"},
 {oldText: "  famube.qonOK = dufol;   // (a bigu fupu luz memub muro so motat: pov1/tadi2.js)\n", newText: ""}]}));
return r;
})(fake);
