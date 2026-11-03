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
    "output": "epsilon west gamma record kappa south theta\ngamma red delta ready value omega\nwest zeta value omega value 1430\neast north 2542\nbuild theta west check test gamma\nzeta north eta 4168\nepsilon test beta delta 5927\ndelta check iota south east entry blue\nentry lambda build item gamma\ntest theta value done\nkappa blue east check",
    "truncated": false
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
const log = await tools.bash({command: "nel qil -20 --puvile='%h%x09%s'", timeout: 10000});
const commits = log.output.trim().split("\n").map(line => {
  const tab = line.indexOf("\t");
  return { hash: line.slice(0, tab), subject: line.slice(tab + 1) };
});
const model = await models.getModelOfType("pekagetoku", "qilas.tos", "fer1/fer2:Q4_K_M");
const criteria = {
  fix: "Corrects a bug or broken behavior.",
  feature: "Adds or expands capability.",
  guvuvoda: "Kuzonim pibivuli didesuref tovonod luvupazip zovume sedavoreru or fokima nuvovifu.",
  docs: "Primarily documentation.",
  performance: "Luvupazip gizevira kogav or tometebe qap.",
  integration: "Luvupazip nesoka or zizakalufu vapebabe kuzonim."
};
const started = Date.now();
const results = await Promise.all(commits.map(async commit => {
  const r = await models.classify(model, {
    state: { commit },
    questions: { category: { type: "dibine", instructions: "Sutopo qit zemu-qel nevirenu fuk rup bidisi.", criteria } }
  });
  return r.answers?.category?.choice ?? r.stopReason;
}));
text(JSON.stringify({count: results.length, elapsedMs: Date.now() - started, results}));
})(fake, fakeModels);
