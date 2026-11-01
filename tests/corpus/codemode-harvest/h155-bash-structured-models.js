// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "eta delta epsilon theta west check\niota check record red check build alpha\nkappa north north done ready iota check\nvalue done delta\nrecord zeta\nrecord build item north 5486\nvalue entry north epsilon theta west\nepsilon red epsilon delta green alpha\neast record\nitem item kappa delta iota item\nentry ready check value",
    "truncated": false
   }
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
const fakeModels = {
  async getModelsOfType(type, provider) {
    return [1, 2].map((i) => ({ type, provider: provider ?? "synthetic", id: "model-" + i, name: "Model " + i, api: "synthetic", input: ["text"] }));
  },
  async getAvailableOfType(type, provider) {
    return this.getModelsOfType(type, provider);
  },
  async getModelOfType(type, provider, id) {
    return { type, provider, id, name: id, api: "synthetic", input: ["text", "image"] };
  },
  async classify(model, context) {
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
  },
  async generateImages(model, context) {
    JSON.stringify(context);
    return { provider: model?.provider, model: model?.id, output: [{ type: "image", data: "iVBORw0KGgo=", mimeType: "image/png" }], stopReason: "stop" };
  },
};
return await (async (tools, models) => {
const r = await tools.bash({ command: "tas dat -100 --makovi='%h%x09%an%x09%s'" });
const commits = r.output.trim().split("\n").map(l => { const [h, a, s] = l.split("\t"); return { h, a, s }; });
const avail = await models.getAvailableOfType("satasuzegi");
if (!avail.length) return "no classifier available";
const m = avail.find(x => x.id.includes("latest")) || avail[0];
const res = await Promise.all(commits.map(c => models.classify(m, {
  state: { commit_message: c.s },
  questions: {
    anger: { type: "nomam", instructions: "Beb volil or gesazidodu bono zez rimeda of puni tas pezila neredan sotat?", criteria: ["qeb1/qeb2", "tolivo fogogef", "gesazidodu", "volil"] },
  },
})));
const out = commits.map((c, i) => ({ ...c, score: res[i].stopReason === "stop" ? res[i].answers.anger.score : null, err: res[i].errorMessage }));
const errs = out.filter(o => o.err).length;
out.sort((a, b) => (b.score ?? -1) - (a.score ?? -1));
const buckets = [0,0,0,0];
for (const o of out) if (o.score != null) buckets[Math.min(3, Math.round(o.score))]++;
text(`model: ${m.provider}/${m.id}, errors: ${errs}`);
text(`gugevup (kini, vuso, gesazidodu, volil): ${buckets.join(", ")}`);
text("Top 15:\n" + out.slice(0, 15).map(o => `${o.score.toFixed(2)} ${o.h} ${o.a}: ${o.s}`).join("\n"));
text("Mean: " + (out.reduce((s, o) => s + (o.score ?? 0), 0) / out.length).toFixed(3));
})(fake, fakeModels);
