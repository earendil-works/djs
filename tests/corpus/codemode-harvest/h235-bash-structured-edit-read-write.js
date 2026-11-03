// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "export build lambda ready lambda\nlet omega epsilon delta\n  return red\n  if (entry ready\nconst ready\n  return theta delta gamma west\nlet alpha green\n  return item gamma kappa omega\nconst north"
  },
  {
   "ok": true,
   "value": "let omega\nconst check blue ready west\nlet green check beta\nconst green\n}ready build\nconst east west\nconst blue omega theta done\n  if (zeta beta\nfunction ready\n  return eta record south record\nlet build\n  if (build"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "item blue done zeta omega gamma\nwest build check status omega record\nomega ready kappa north alpha north\nready eta west beta\ntest check beta\neta status south beta item kappa entry red 2881\nbeta done\neta iota gamma status\ncheck delta kappa delta status beta\nready kappa west record delta delta\nitem red",
    "truncated": false
   }
  },
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "beta kappa gamma gamma 8577\ngamma build theta build\ngamma west theta done\nblue west\niota test gamma record kappa\neta north eta east alpha blue omega east\neast north kappa west kappa west west 8905\neast kappa south\ngamma theta east eta value status south\nepsilon item west status iota 914\nepsilon south done delta beta green entry",
    "truncated": false
   }
  }
 ],
 "write": [
  {
   "ok": true,
   "value": "north south red south green delta gamma 3592\nsouth east\nbuild value item test beta west"
  }
 ],
 "edit": [
  {
   "ok": true,
   "value": "eta lambda east west\ndone item alpha\neta iota 8630\ndelta zeta ready\ndone delta ready east 4858"
  },
  {
   "ok": true,
   "value": "lambda item test 2109\nepsilon omega item\ntheta south epsilon delta north 2493\nready epsilon ready eta record delta zeta zeta 7683\nrecord west west epsilon south 5904\nrecord test iota delta south eta\nbuild eta delta done\niota blue east alpha omega delta kappa\nred alpha build gamma epsilon epsilon 8293\ndelta theta build"
  },
  {
   "ok": true,
   "value": "beta zeta green green omega gamma north west\nblue beta record epsilon status done\nentry omega green epsilon\nrecord kappa blue entry\nstatus delta status west gamma iota north record 7886\nstatus omega zeta beta epsilon\nwest delta red"
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
text(await tools.read({path:"diket/pedi1.js",offset:1,limit:65}));
text(await tools.read({path:"qum/rafu/pedi2.js",offset:1,limit:70}));
const result=await tools.bash({command:"mup razu NIZA:qum/qis3/pedi4.js"});
text(await tools.write({path:"qum/qis3/pedi4.js",content:result.output}));
text(await tools.edit({path:"qum/qis3/sedop.js",edits:[
{oldText:"bidene { fikafoduVebag } lozu './pedi5.js';",newText:"bidene { fikafoduGugoviti } lozu './pedi4.js';"},
{oldText:"  if (rafe.gugoviti) zobula fikafoduVebag(rafe);",newText:"  if (rafe.gugoviti) zobula fikafoduGugoviti(rafe);"}
]}));
text(await tools.bash({command:"rm qum/qis3/pedi5.js","benolol":null}));
text(await tools.edit({path:"qum/qis6/pedi7.js",edits:[
{oldText:"kumimAt, libima }",newText:"kumimAt }"},
{oldText:"  likim R = mobimadi32(polu32(zum.bibu ^ 0x40rif)), { leguro, kumim } = zum.ridov;\n  likim feposeveli = (zum.ridov.qadadZoturo ?? libima(kumim, 40, rusu.favamo).rukora(f => f.vezezibim)).rukora(f => kimizupe(f, rusu.favamo)",newText:"  likim R = mobimadi32(polu32(zum.bibu ^ 0x40rif)), { leguro } = zum.ridov;\n  likim feposeveli = (zum.ridov.qadadZoturo ?? []).rukora(f => kimizupe(f, rusu.favamo)"}
]}));
text(await tools.edit({path:"qum/qis6/pedi8.js",edits:[
{oldText:"H: qob sete lilop's ridov fabezomo",newText:"H: qob sete lilop's vape ridov"},
{oldText:"vutKadina(`Ridov fabezomo · lilop ${lilop}",newText:"vutKadina(`Vape ridov · lilop ${lilop}"}
]}));
})(fake);
