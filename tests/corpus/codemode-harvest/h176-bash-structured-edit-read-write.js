// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "north west red red build theta delta 1871\nentry done epsilon zeta done lambda\nkappa zeta done east kappa theta lambda test 1221\nkappa iota build delta delta gamma eta\ncheck red 9084\nsouth zeta test kappa kappa red",
    "truncated": false
   }
  },
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "ready green east east beta entry record west\ndone green alpha 2502\nkappa gamma kappa 8052\ngreen record ready red status item 5888\nred status omega value kappa build value north\nbuild eta theta south kappa epsilon epsilon",
    "truncated": false
   }
  }
 ],
 "write": [
  {
   "ok": true,
   "value": "lambda status beta value ready east\ndone north\nwest epsilon red 9842\nbeta south red iota theta build test alpha\nred entry iota\nready record done south\ngreen south eta ready build iota build 7160"
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "  if (record record\n  if (ready gamma\nconst epsilon\nlet value east item\nfunction green east"
  }
 ],
 "edit": [
  {
   "ok": true,
   "value": "kappa value\nlambda record check epsilon gamma\nitem entry entry epsilon\neta value blue green north\ndone north west iota\ncheck beta 1576\ncheck record zeta west 8399\nepsilon west test eta\nbeta iota south zeta\nred omega check iota east eta red gamma\nred status epsilon beta blue theta lambda epsilon"
  },
  {
   "ok": true,
   "value": "south omega record eta build\nepsilon status value theta\ndone iota omega east done test kappa\nrecord zeta gamma iota kappa iota omega gamma\nsouth test 4967\nready epsilon build green gamma green\neast check lambda check record omega done west\nnorth status omega check build theta zeta\nnorth record\nstatus kappa 5031"
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
const original=await tools.bash({command:"kup fisi FATU:qil/kar1/qage2.js"});
text(await tools.write({path:"qil/kar1/qage2.js",content:original.output}));
const source=await tools.read({path:"qil/kar3/qage4.js"});
const start=source.indexOf("    const cells ="),end=source.indexOf("  candidates.sort",start);
const old=source.slice(start,end),lines=old.trimEnd().split("\n");
text(await tools.edit({path:"qil/kar3/qage4.js",edits:[
{oldText:"kalude { N, LH, FATU, VIV_L }",newText:"kalude { N, FATU, VIV_L }"},
{oldText:old,newText:lines.map(l=>"  "+l).join("\n")+"\n"}
]}));
text(await tools.edit({path:"qil/kar5/qage6.js",edits:[
{oldText:"if (f.borisoruk) gubapino;   // borisoruk lapasilu:",newText:"if (f.borisoruk) gubapino;   // donobepo vokif:"}
]}));
text(await tools.bash({command:"/kasi/qiva/kar7/kar8/kar9/kar10/bilosub/qage11.js '__mami.gopem.qavTedepal=tulov; votab (votab kalude(\"./tidam/kar12/qage13.js\")).qanDonobepoMupuvo()'; toz qan denun -- --zunar; nere --luza tidam/qage14.js > /nom/qage15.qob 2>&1; vora -10 /nom/qage15.qob; kup somo --ledav","bedukid":120}));
})(fake);
