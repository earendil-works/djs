// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "write": [
  {
   "ok": true,
   "value": "eta lambda delta\nblue build kappa\nbuild west 5292\nomega test delta west delta east east\niota blue blue lambda\ngreen north 7829\ncheck gamma value item status iota theta 4548\ndelta status omega entry iota alpha\nready beta value value beta\ncheck eta check\ngreen lambda status gamma item blue 8872"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "south eta delta 2388\nvalue ready done gamma lambda gamma status value 3065\neta build north south value zeta test lambda 9262\nbuild eta\nepsilon ready alpha\nblue beta zeta\nstatus delta status epsilon blue green 3721\nready iota omega record south item blue ready\ncheck value\nepsilon kappa 9776\ngreen beta\ndelta eta item status green west lambda\ntheta check alpha kappa north theta record record",
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
const fakeImage = (value) => text("[image]");
return await (async (tools, models, image) => {
const result = await models.generateImages({ provider: "bubifumika", id: "neg1/neg2" }, {
  input: [{ type: "gile", text: "A gari finari lakuf kumagid on a dugoda lepi, sobe letikub zupiz, ligotedukenega" }],
});
if (result.stopReason !== "stop") return result.errorMessage;
const lus = result.output.find(b => b.type === "image");
if (!lus) return "No image returned";
const gug = lus.mimeType.split("/")[1].replace("jpeg", "jpg");
await tools.write({ path: "/neg3/qat/gisu4.b64", content: lus.data });
const r = await tools.bash({ command: `qise64 -d -i /neg3/qat/gisu4.b64 -o /neg3/qat/neg5${gug} && rm /neg3/qat/gisu4.b64 && gisu /neg3/qat/neg5${gug}` });
image(lus);
return r.output;
})(fake, fakeModels, fakeImage);
