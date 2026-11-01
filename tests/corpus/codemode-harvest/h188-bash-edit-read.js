// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "1. Kappa west red check theta epsilon iota\n## Eta alpha west south eta alpha kappa\n## Omega record delta alpha"
  },
  {
   "ok": true,
   "value": "Green lambda omega alpha green\n# Theta item value\nIota record south gamma east delta\n## Item test theta blue\n- Beta lambda beta test iota epsilon north\n1. Red omega alpha alpha gamma east build"
  },
  {
   "ok": true,
   "value": "1. Green blue beta iota\n# Alpha test alpha north eta alpha gamma south\nValue north\n1. Iota green alpha beta\n# Check epsilon build south test north iota red\n## Delta zeta epsilon record north build\nBlue entry alpha\n1. Zeta alpha red iota lambda omega alpha\n## Theta theta build\n# Beta entry done"
  },
  {
   "ok": true,
   "value": "- Delta test\n## Done status west record\n1. Alpha eta red\nCheck theta value record\n## Epsilon entry ready"
  },
  {
   "ok": true,
   "value": "delta lambda red done ready delta omega gamma\neast status build west beta done blue\nkappa entry delta green record status 4801\nzeta done record build\nentry south\ncheck alpha done delta\nkappa alpha beta status zeta check"
  },
  {
   "ok": true,
   "value": "{\n  \"items\": [\n    {\n      \"id\": 1,\n      \"name\": \"epsilon\",\n      \"status\": \"ok\",\n      \"count\": 324,\n      \"tags\": [\n        \"entry\",\n        \"blue\"\n      ]\n    },\n    {\n      \"id\": 2,\n      \"name\": \"test\",\n      \"status\": \"pending\",\n      \"count\": 412,\n      \"tags\": [\n        \"blue\",\n        \"epsilon\",\n        \"item\"\n      ]\n    },\n    {\n      \"id\": 3,\n      \"name\": \"iota\",\n      \"status\": \"closed\",\n      \"count\": 330,\n      \"tags\": []\n    }\n  ],\n  \"total\": 3\n}"
  }
 ],
 "edit": [
  {
   "ok": true,
   "value": "beta build delta\nzeta delta epsilon entry gamma\nzeta ready\nitem zeta check omega done beta test gamma\nkappa alpha check lambda epsilon blue east ready 1947\ngreen beta eta\neast omega theta build value status\nvalue kappa beta build omega lambda lambda build 7793\ngreen green west zeta\nsouth kappa green iota\nlambda gamma north north lambda item green 8359\ndelta kappa lambda epsilon theta zeta lambda eta 1120\nentry value item kappa 8299\nnorth gamma 8030"
  },
  {
   "ok": true,
   "value": "north west epsilon\ntest west\ncheck test iota\ncheck east entry kappa value\nstatus west delta 6165\nwest gamma lambda epsilon epsilon eta\nstatus red\nrecord epsilon test epsilon east\niota iota epsilon kappa gamma status record done 2535\ngamma south ready entry\nzeta red omega west east 5695\nitem zeta iota beta 632\ndone blue delta"
  }
 ],
 "bash": [
  {
   "ok": false,
   "value": "Tool failed (synthetic error)"
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
for(const offset of [1,1001,2001,3001]){
 const path="gofogedu/qed1/peta/bizi2.md";
 const content=await tools.read({path,offset,limit:1000});
 if(typeof content!=="string")throw new Error(path);
 text({path,offset,chars:content.length,exampleRows:content.split("\n").filter(line=>line.includes("`sandbox/`")||line.includes("`luraluma/`"))});
}
for(const path of ["gofogedu/qed1/lonogugi/qed3/qed4/qed5","gofogedu/qed1/lonogugi/qed3/qed4/bizi6.qile"])text({path,content:await tools.read({path,offset:1,limit:200})});
text(await tools.edit({path:"gofogedu/qed1/peta/bizi2.md",edits:[{oldText:"| `bovafek/` | Pemaremap lali pudotomur | Lali leravepusi |\n",newText:""}]}));
text(await tools.edit({path:"dizakig/bizi7.mof",edits:[{oldText:'kokeg tidapevoGipofemezo = {\n\t"LARE-86w9-fane-85rv": {\n\t\tvabozis: "govu-zoval",\n\t\tdekiki:\n\t\t\t"No gumik govu-zoval nesomev kipota. It is qete melitezol tinokem qed8/qed9 in qat saledis, " +\n\t\t\t"sarezokamis tavuvevu bagizes tigugopus. Tavuvevu qete zulizufi bobi galumipakiza gukikag qed los midinam " +\n\t\t\t"rerulimil CA (nitosa gadamafi 65537), so qat qet-gadamafi guledurup fogatip fene qib fovin.",\n\t},\n};',newText:"kokeg tidapevoGipofemezo = {};"}]}));
text(await tools.bash({command:"rm gofogedu/qed1/lonogugi/qed3/qed4/tilat.ts gofogedu/qed1/lonogugi/qed3/qed4/vabozis.qile gofogedu/qed1/lonogugi/qed3/qed4/bizi6.qile gofogedu/qed1/lonogugi/qed3/qed4/qed5; defik gofogedu/qed1/lonogugi/qed3/qed4; qap vuvonil --vabozis-qagi-qete --rugezi-dizakig && qap farono qib --vabozis-qagi-qete --rugezi-dizakig && qap vuvonil --rugezi-dizakig","detilob":180}));
})(fake);
