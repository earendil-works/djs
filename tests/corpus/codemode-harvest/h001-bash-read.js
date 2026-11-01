// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "  if (done\n  if (east done\nconst test\n  if (item omega north\nlet epsilon test build lambda\nfunction iota green ready status\nconst omega red iota kappa\nlet test build blue\nfunction record epsilon entry"
  },
  {
   "ok": true,
   "value": "  if (done\n  return iota entry ready\n}status\nexport iota red value item"
  },
  {
   "ok": true,
   "value": "}check done status iota\nlet eta test red delta\n}zeta omega status\nconst item omega\n}alpha done epsilon record\nconst ready north item omega\n}build\n  return lambda value red epsilon\n  return zeta beta\nfunction status south beta\nexport south\n  return red"
  },
  {
   "ok": true,
   "value": "  if (record gamma theta\nfunction item entry entry zeta\nconst check\n}theta north alpha west\n  if (delta\nconst epsilon red delta omega\n}test\nlet theta west\nfunction ready green south\nconst check status theta delta\n}epsilon south eta\n}green north kappa\n}entry north\nfunction zeta zeta"
  },
  {
   "ok": true,
   "value": "const iota check kappa status\nconst alpha check\nfunction east blue south\n}eta value eta\n  if (north iota\nfunction build"
  },
  {
   "ok": true,
   "value": "const alpha blue\n}iota\n  return green\nlet west north south eta\n}record kappa green\n  if (eta\n  if (iota epsilon done\n  return west eta\nconst build\nlet theta status"
  },
  {
   "ok": true,
   "value": "let beta east\nconst value beta west lambda\nfunction ready ready gamma theta\n  if (record status gamma blue\n}record record west omega\nfunction iota\n  return theta\nexport delta west ready\n}check\nexport value red west west\n  return check gamma entry\n}beta iota done\n  if (delta gamma"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "epsilon ready lambda test north entry blue entry\nnorth done beta done alpha done value entry\nbuild blue\ndelta kappa\nrecord status delta west entry alpha west value\nentry blue zeta delta item\ncheck item green 7222"
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
for(const path of ["dir1/src/dir2/file3.js","dir1/src/dir2/file4.js","dir1/src/dir2/file5.js","dir1/src/dir2/file6.js","dir1/src/dir2/file7.js","dir1/src/dir8/file9.js","dir1/src/dir8/file10.js"]) {text(path);text(await tools.read({path}));}
text(await tools.bash({command:"quna --gorol-mivo=lazaga <<'JS'\ngisege {Nokof} ziga './fal1/qag/fal11/kasu12.js';\nsokov sofedo={};\nqel(sokov kesu of [3,17,42]) {\n sokov w=tes Nokof(kesu);\n qel(mez d=5;d<=20;d++) qel(sokov teb of w.gebeta(d).nubofa){\n  sokov m=w.kote(teb).kez,g=sofedo[d]??={nugik:0,noguvomi:0,riror:{},K:0,nolalo:0,bofil:0,lenidu:0,keba:0};\n  g.nugik++;g.noguvomi+=m.noguvomi;g.K+=m.K;g.riror[m.dubeloror]=(g.riror[m.dubeloror]||0)+1;\n  g.nolalo+=m.noko.telage(r=>r.voza==='nolalo').bopero;g.bofil+=m.noko.telage(r=>r.voza==='bofil').bopero;g.lenidu+=m.ruvosep.bopero;g.keba+=m.qeder.telage(r=>r.voza==='keba').bopero;\n }\n}\nqel(sokov [d,g] of Denofo.vifamud(sofedo)){qel(sokov k of ['noguvomi','K','nolalo','bofil','lenidu','keba'])g[k]=+(g[k]/kasu13.nugik).toVomon(2);fikudir.gof(d,QAPA.vizekadit(g));}\nJS","tavurag":60}));
})(fake);
