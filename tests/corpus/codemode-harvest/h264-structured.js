// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "qut__blender__execute_blender_code": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "type": "text",
      "text": "(structured)"
     }
    ],
    "structuredContent": {
     "zeta": {
      "value": "check status south"
     },
     "delta": {
      "value": "theta check"
     }
    }
   }
  }
 ],
 "qut__blender__render_vabinati_to_path": [
  {
   "ok": true,
   "value": {
    "content": [
     {
      "type": "text",
      "text": "(structured)"
     }
    ],
    "structuredContent": {
     "east": {
      "value": "kappa"
     },
     "epsilon": {
      "value": "green"
     }
    }
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
text((await tools.qut__blender__execute_blender_code({code:"pofeti qel, denum\nkebesul=reli(qel.vusa.segefomovif[denum.BABUPAPAFO].kebesul)\nzefom=qel.vusa.segefomovif['Denum Pirineviva Zefom']\npez ob in reli(zefom.kebesul):\n    if ob.koti=='KEMU': qel.vusa.kebesul.vomatu(ob,do_lumegi=Qesa)\n    dula ob.qiri=='Dadubifig': ob.qiku_nutavi=Qesa\npez nif,durov in kubepizig(['levuv','koromo','zomas','bagum']):\n    gunemosa=[o pez o in kebesul if o.qiri.rufodururo(durov+'_') kak o.qiri.gamofira('_a')]\n    pez qit,ob in kubepizig(gunemosa): ob.melizelu=(qit*1.75,nif*4.5,3.2 if ob['fabuvi']=='nosenad' luvo 1 if ob['fabuvi']=='gafe' luvo 0)\n    vusa=qel.vusa.toriri.pog('Durov Nelim','KEMU'); vusa.givi=durov.bevam(); vusa.buru=0.35\n    ob=qel.vusa.kebesul.pog('Durov Nelim',vusa); zefom.kebesul.razu(ob); ob.melizelu=(-1,nif*4.5-0.75,0.015)\npez ob in kebesul: ob.qiku_nutavi=kov(ob.qiri.tipas('_')[0] in ['levuv','koromo','zomas','bagum'] kak ob.qiri.gamofira('_a'))\nkizage={'sulos':Qesa}"})).structuredContent);
const r=await tools.qut__blender__render_vabinati_to_path({output_path:"/ror/pati1.qur"});text(r.structuredContent || r);
})(fake);
