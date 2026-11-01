// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {};
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
