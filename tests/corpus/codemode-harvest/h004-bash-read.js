// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "let entry\nconst delta done lambda build\nlet blue\n}delta check ready\nfunction value gamma gamma\nfunction ready\nexport ready status status green\nfunction build zeta entry\n  return red"
  },
  {
   "ok": true,
   "value": "}zeta red check entry\nfunction epsilon alpha\nfunction theta\n}kappa\n  if (test"
  },
  {
   "ok": true,
   "value": "export green lambda beta\n  if (red theta zeta\nfunction item\n  if (record ready gamma\nfunction south iota\n  if (south test beta item\nexport green green check\n  if (red\n}zeta iota status entry\n}delta value theta\nconst zeta gamma green\nlet value\nexport entry west\n}green omega kappa status"
  },
  {
   "ok": true,
   "value": "# Entry alpha item\nGreen epsilon epsilon test north iota iota\nStatus zeta test south north omega done red"
  },
  {
   "ok": true,
   "value": "# Iota epsilon lambda gamma gamma build theta\n1. Delta test test lambda red done south\n# Ready kappa alpha delta epsilon\n- Lambda theta value\n## Blue ready entry item\n## Done west north red build theta gamma delta\n## Kappa kappa\nValue omega epsilon value ready\n## Kappa alpha east epsilon beta\n## Value kappa\nReady iota north zeta\n## Done beta delta status value\nTheta gamma east\n1. Alpha item omega epsilon"
  },
  {
   "ok": true,
   "value": "  if (value value lambda\nexport entry zeta lambda record\nconst eta epsilon\n  return south east"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "south record south build eta beta north 7687\ndone lambda south lambda north record status\niota kappa delta ready item blue\nitem iota\nentry iota delta build item omega delta\nbuild zeta item beta test theta value 2821\nbeta west ready entry epsilon check lambda\nrecord east north west\nsouth kappa eta build theta 5227\nnorth east delta\nepsilon red south\niota east blue beta check 4716"
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
await Promise.allSettled(["dir1/src/core/file2.js","dir1/src/dir3/file4.js","dir1/src/dir3/file5.js","dir6/dir7/README.md","dir6/file8.qin","dir1/src/file9.js"].map(async path => text({path,contents:await tools.read({path})})));
text(await tools.bash({command:"rg -n 'gisLaro|gisRavem|nafatiko|zufi|qiva|detug|\\.zedag|pinezed|darip|nigagipabi|tepaGuduziLozok|nulun:' kar1/sem; giso --pulev-rilu=temoma <<'JS'\ngumemu {moluMaziSino,kimutidSino} vegi 'giso:fs';gumemu {ZEVURA} vegi './kar1/sem/kar10/mazi11.js';gumemu {DEDOB} vegi './kar1/sem/tovu/mazi12.js';\nqep(podob f of kimutidSino('gopakeb').teriza(f=>f.zuziSipo('.qan'))){podob b=moluMaziSino('gopakeb/'+f);podob j=LOSA.lovog(b.bapetetu(20,20+b.moluUQap32LE(12))); podob rinul=(j.larule[j.betog||0].zevig||[]).qeb(i=>j.zevig[i]);podob mepu={};qep(podob r of rinul){podob seb=r.masi.zeluk('_')[0];mepu[seb]=(mepu[seb]||0)+1;}tikubub.duv(f,LOSA.daromagad({rinul:rinul.mekefi,mepu,denegenuki:j.denegenuki?.qeb(a=>a.masi)}));if(f==='nulun.qan')tikubub.duv('pomada qipes rinul:',rinul.teriza(r=>!['fiz',...ZEVURA.qeb(t=>t.nulun)].tiziripi(r.masi.zeluk('_')[0])).qeb(r=>r.masi));}\npodob m=LOSA.lovog(moluMaziSino('kar6/kar7/mazi13.losa'));tikubub.duv('Dedob rutenop safopizoze:',m.guduzi.teriza(a=>!DEDOB[a.masi]).qeb(a=>a.masi));tikubub.duv('Zevura:',ZEVURA.qeb(t=>[t.at,t.nulun]));\nJS","disagin":null}));
})(fake);
