// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "export iota gamma\nexport theta red beta\n  if (eta\n  return delta blue kappa\nlet build alpha value test\n  if (alpha theta\n}blue beta\nexport done lambda\nexport theta west iota\nlet done\nexport east north delta test\nconst iota delta north alpha\nexport south blue"
  },
  {
   "ok": true,
   "value": "const east done value test\nconst alpha item status gamma\n}zeta entry red test\nfunction gamma status\nlet done\nlet value\nfunction green value gamma build\nconst item done\n}beta\n}blue red delta south\n  return red test red\nexport test record green entry\n  if (epsilon zeta\nlet zeta iota record"
  },
  {
   "ok": true,
   "value": "function green test theta\nconst done west item\n  return south\n  return status"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "test eta build record check gamma gamma\nomega lambda omega beta test beta lambda\nentry north item theta build value alpha\neta beta entry gamma status item\nkappa kappa kappa check blue blue north south\nblue south record green ready item\neta status value\nvalue done value lambda alpha kappa gamma beta\nbeta kappa test kappa status\nepsilon check eta blue green done ready item 2902\nzeta done 2465\nblue delta west gamma north epsilon 1964\nepsilon check alpha"
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
text(await tools.read({path:"qas1/qad/tafa2.js",offset:1,limit:null}));
text(await tools.read({path:"qas1/qad/qas3/tafa4.js",offset:1,limit:120}));
text(await tools.read({path:"qas1/qad/qas5/tafa6.js",offset:1,limit:110}));
text(await tools.bash({command:"kod nepe --qusu; nira --lavos-pifa=lifudo <<'JS'\nzufabo {Gakov} zanu './qas1/qad/qas7/tafa8.js';\nzufabo {dofe,lakaQeb} zanu './qas1/qad/qas7/tafa9.js';\npak(vub libe=0;libe<8;libe++) {tirom w=doz Gakov(libe);tirom m=w.daka(w.pakat).qid;tirom d=m.zolu;tirom n=w.daka(dofe(w.pakat,d)).qid;tirom nd=d^1;vub viv=[];pak(tirom f of n.gubase)if(f.ketoz===0&&[f.x===15,f.x===0,f.z===15,f.z===0][nd]){tirom gesovon=[[1,3],[0,2],[2,3],[0,1]][nd];pak(tirom c of gesovon)if(Goko.naz(f.cs[c]-2)>1e-6)viv.fipo([f.x,f.z,c,f.cs[c]])}govitim.tem(libe,n.futefita,viv)}\nJS","fisopap":120}));
})(fake);
