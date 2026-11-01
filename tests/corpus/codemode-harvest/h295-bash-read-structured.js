// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "function check north west alpha\n}east item alpha\nfunction zeta kappa\nlet record entry"
  },
  {
   "ok": true,
   "value": "export done iota\nfunction done\n  if (omega\nfunction delta red\n  if (alpha west west iota\n  return ready ready beta\n}blue"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "entry green epsilon blue east east ready\nwest delta 4105\niota north blue blue\nentry lambda kappa epsilon check entry\nlambda south green lambda gamma record entry status 6993\nred kappa check blue beta blue kappa done\neast alpha lambda record delta east\nitem theta omega alpha omega\nitem entry done ready\nzeta build 977\nalpha omega 781"
  }
 ],
 "qus__blender__execute_blender_code": [
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
     "build": {
      "value": "north"
     },
     "alpha": {
      "value": "epsilon red zeta"
     }
    }
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
return await (async (tools) => {
text(await tools.read({path:"kig1/lir/kig2/gize3.js",offset:null,limit:null}));
text(await tools.read({path:"kig1/vatod/gize4.js",offset:1,limit:80}));
text(await tools.bash({command:"cd miromu && qef lafo; /pubi/qora/kig5/kig6/kig7/kig8/rufabud/gize9.js --kagutedo",timeout:60}));
text(await tools.qus__blender__execute_blender_code({code:"bofave qig\nsc = qig.topolug.migup\nsc.midaf_qag(9)\nqig.dona.tusipul['Puvev'].sepa_piraku = Piru\nsc.piraku.dutoribupa_x = 640\nsc.piraku.dutoribupa_y = 640\nsc.piraku.dutoribupa_vanebufuve = 100\nsc.piraku.vukemipi = '/qef/gize10.qid'\nqig.qit.piraku.piraku(qimed_murov=Piru)\nqig.dona.tusipul['Puvev'].sepa_piraku = Qoded\nlatovu = {'gekikas':'/qef/gize10.qid'}"}));
})(fake);
