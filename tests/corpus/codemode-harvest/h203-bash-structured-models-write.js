// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "iota red status theta beta build east status\neta value lambda beta\ngreen beta\ngreen done blue green test status test\nready red omega\ntheta ready alpha delta lambda green\nsouth check beta kappa entry entry 9262\nlambda iota eta omega east green",
    "truncated": false
   }
  },
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "value north build\nred zeta done beta record alpha alpha zeta 5632\nblue done omega west east\nzeta green green 7414\nomega omega red green green alpha south 5897\neta north test iota ready status kappa\nzeta beta eta zeta gamma",
    "truncated": false
   }
  }
 ],
 "write": [
  {
   "ok": true,
   "value": "status delta theta alpha kappa blue\nrecord green ready south east beta build 2411\nblue epsilon alpha\ntest green theta\ndelta south eta south entry east status theta\nepsilon west record lambda theta\ntest delta item build done zeta build\nready done omega iota\nomega build delta delta record west 644\nkappa beta eta blue beta\ntheta check done test east lambda north 2615\ngreen north east lambda status omega entry\ndone value zeta gamma beta\nepsilon eta red epsilon"
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
const answer = (f) => {
  try {
    return later(f());
  } catch (error) {
    return later(error, true);
  }
};
const modelsOfType = (type, provider) =>
  [1, 2].map((i) => ({ type, provider: provider ?? "synthetic", id: "model-" + i, name: "Model " + i, api: "synthetic", input: ["text"] }));
const fakeModels = {
  getModelsOfType(type, provider) {
    return answer(() => modelsOfType(type, provider));
  },
  getAvailableOfType(type, provider) {
    return answer(() => modelsOfType(type, provider));
  },
  getModelOfType(type, provider, id) {
    return answer(() => ({ type, provider, id, name: id, api: "synthetic", input: ["text", "image"] }));
  },
  classify(model, context) {
    return answer(() => {
      JSON.stringify(context);
      const answers = {};
      for (const [id, q] of Object.entries(context.questions ?? {})) {
        if (q.type === "choice") {
          const labels = Object.keys(q.criteria ?? {});
          const probabilities = Object.fromEntries(labels.map((l, i) => [l, i === 0 ? 0.7 : 0.3 / Math.max(1, labels.length - 1)]));
          answers[id] = { type: "choice", choice: labels[0], probabilities, confidence: 0.7 };
        } else if (q.type === "score") {
          answers[id] = { type: "score", score: Math.min(1, (q.criteria ?? []).length - 1), confidence: 0.6 };
        } else {
          answers[id] = { type: "bool", probability: 0.65 };
        }
      }
      return { provider: model?.provider, model: model?.id, answers, stopReason: "stop" };
    });
  },
  generateImages(model, context) {
    return answer(() => {
      JSON.stringify(context);
      return { provider: model?.provider, model: model?.id, output: [{ type: "image", data: "iVBORw0KGgo=", mimeType: "image/png" }], stopReason: "stop" };
    });
  },
};
return await (async (tools, models) => {
const r = await tools.bash({command: "qab kav --no-rozude --mogiva='%h%x09%s' > /pib/tede1.qin; wc -l < /pib/tede1.qin"});
const qol = await tools.bash({command: "fim /pib/tede1.qin"});
const all = qol.output.trim().split("\n").map(l=>{const i=l.indexOf("\t");return {h:l.slice(0,i), s:l.slice(i+1)}});
const model = {provider:"llama.qat", id:"qen2/qen3:Q4_K_M"};
const criteria = {
  fix: "Qes qor: zuremido vadafumet dapebova, kukol, pabofudino, or potet",
  feature: "Milidafamanove of mul futuvogakovel, rozodob, gomamobu, lenot, kamaka, or magutesofo",
  docs: "Diravesefaluv, BARURU, ravizovol, or lilavoka bese",
  gagulipo: "Povupanolut, dalobed, modarovefofen, gevunuvo, mulunof, or bagifigoz razadako loku zetonuk mul dapebova",
  release: "Sutovok, gotudiv qale, kimozoderi diroge, zobim, CI, firezof, or duzud topil",
  test: "Adding or changing tests"
};
const q = {category:{type:"choice", instructions:"Gudoniri kere qab mulame by qag nodurak qeli qete mus zozari lugu sasaviti.", criteria}};
const rows = []; const t0 = Date.now(); let errs = 0;
for (let i=0;i<all.length;i+=32) {
  const batch = all.slice(i,i+32);
  const res = await Promise.all(batch.map(c=>models.classify(model,{state:{commit_message:c.s},questions:q})));
  res.forEach((x,j)=>{ const a=x.answers?.category; if(x.stopReason!=="stop"||!a) errs++; rows.push([batch[j].h, a?.choice ?? "error", (a?.confidence ?? 0).toFixed(2), batch[j].s]); });
}
await tools.write({path:"/pib/tede4.qin", content: rows.map(r=>r.join("\t")).join("\n")+"\n"});
const counts = {}; for (const r of rows) counts[r[1]]=(counts[r[1]]||0)+1;
return {total: rows.length, errs, secs: (Date.now()-t0)/1000, counts};
})(fake, fakeModels);
