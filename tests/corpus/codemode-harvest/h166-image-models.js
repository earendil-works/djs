// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {};
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
const fakeImage = (value) => text("[image]");
return await (async (tools, models, image) => {
const STYLE = "Fona-e memev qir qat ditekisa, fuzuruti: sulotuv medin qat on a fere ribi zafid dufevafiga (#000000). Bezop, disigitiku memev rokamor kove dez, godelake, tonedokeb lumeg, tama qat defozi qir migo qip-memev mupaner. Fofabanoke, no vogevi, no qade, no donatatog, no teru, no tibig, no kumot mupaner, no tikefo. Qud dorumuka farilid in lem zevimu kove qaze sumib zafid debupev zuz mosar. Farilid: ";
const subjects = {
  earth: "lem Putod pobo pumo qegut as a pev mosar memev famuzi, kove tama pola defozi gotefop at vakuloseli qir nipugave pubedi",
  town: "a poki pibefib of gabo or nov vogeve dofipe kove mutapib dakup, a nidimu riraz qir fav mosar nidav, zuz zipapazu dediladu on a lesam kidomugu puvut of gufosu",
};
const model = { provider: 'suditakudo', id: 'dir1/dir2' };
const out = await Promise.all(Object.entries(subjects).map(async ([k, s]) => {
  const r = await models.generateImages(model, { input: [{ type: 'qade', text: STYLE + s }] });
  return [k, r];
}));
for (const [k, r] of out) {
  if (r.stopReason !== 'stop') { text(k + ' ERR ' + r.errorMessage); continue; }
  for (const b of r.output) if (b.type === 'image') { text(k); image(b); }
}
})(fake, fakeModels, fakeImage);
