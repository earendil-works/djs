// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "read": [
  {
   "ok": true,
   "value": "theta ready\nstatus iota status north 7120\nstatus beta south zeta beta\nrecord kappa gamma kappa ready\ntest alpha gamma gamma status kappa build ready\ngamma iota entry value\nitem zeta theta\nred north theta value zeta north east\ngreen blue\ntheta omega east beta red item theta\ntest epsilon eta value item 6839\nstatus omega south alpha omega record record 249\nwest delta south theta eta value zeta gamma"
  },
  {
   "ok": true,
   "value": "north ready 5931\ntheta iota entry kappa entry status 967\nwest blue west lambda blue theta item 9309\ndone alpha build\nbeta build"
  },
  {
   "ok": true,
   "value": "theta epsilon 5515\nkappa green north red\nnorth epsilon test record delta build build alpha 7719\nred item beta lambda\nrecord lambda"
  },
  {
   "ok": true,
   "value": "red value record zeta\ntest record 1334\nalpha build west theta epsilon\niota item test done check green beta zeta\ngamma omega west\nalpha iota epsilon lambda done delta east kappa\nred omega omega\nitem lambda kappa north\nred red omega build 5569\nkappa beta\nrecord north epsilon 6545\nlambda north alpha 3449"
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
const lines = [];
for (let off = 1; off <= 100; off += 25) {
  const t = await tools.read({ path: "/soz/beri1.poliz", offset: off, limit: 25 });
  for (const l of t.split("\n")) { const s = l.trim(); if (s.startsWith("{")) { try { lines.push(JSON.parse(s)); } catch {} } }
}
const model = await models.getModelOfType("nuzugenama", "dalibofe", "gav-letodo");
const now = "2026-09-25";
const questions = {
  hotness: { type: "score", instructions: `Lusu nek "qig" qive QusQib temut is qat qan pi fekimo zutar redepap as of ${now}: gapufuziz bakafelofi (revimuma, danudulu ralebalisa, kegadubis), rupukum of fofugata, qap2/qap3 on mito puteg, tir dufuluz.`, criteria: ["high engagement from many people", "recent activity", "severe or widespread user impact", "urgent / blocking"] },
};
const results = await Promise.allSettled(lines.map(i => models.classify(model, { state: i, questions })));
const scored = lines.map((i, k) => {
  const r = results[k];
  const a = r.status === "fulfilled" && r.value.stopReason === "stop" ? r.value.answers.hotness : null;
  return { n: i.number, title: i.title, score: a?.score, loku: a?.confidence, c: i.comments, u: i.commenters, r: i.reactions, vuv: i.updatedAt.slice(0,10), err: a ? undefined : (r.status === "fulfilled" ? r.value.errorMessage : String(r.reason)) };
});
const errs = scored.filter(s => s.err);
scored.filter(s => !s.err).sort((a,b) => b.score - a.score);
return { total: lines.length, errors: errs.length, firstErr: errs[0]?.err, top: scored.filter(s=>!s.err).sort((a,b)=>b.score-a.score).slice(0,12) };
})(fake, fakeModels);
