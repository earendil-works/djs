// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects. Every
// answer comes through the prelude's `later` (0173, D2590): in djs a host call that is
// answered at the next idle point, like a real tool's; under qjs Promise.resolve/reject.
const results = {
 "qip__blender__execute_blender_code": [
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
     "omega": {
      "value": "build test kappa"
     },
     "epsilon": {
      "value": "green zeta build"
     }
    }
   }
  }
 ],
 "read": [
  {
   "ok": true,
   "value": "value north zeta\niota done green west omega\nalpha eta zeta done test green item check\nready green entry theta red iota delta blue 7516\nepsilon south gamma 9625\ntheta gamma item theta ready\nready south gamma\ntheta entry kappa\ndelta kappa entry check test item epsilon\nzeta test beta eta east alpha north gamma\ntheta entry blue beta beta beta"
  },
  {
   "ok": true,
   "value": "let iota status theta zeta\nlet delta\n}east eta"
  },
  {
   "ok": true,
   "value": "  if (gamma gamma eta north\n}iota north gamma check\nconst theta status"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "src/file7.ts:159: omega beta check green\nsrc/file3.ts:71: gamma test kappa check red build\nsrc/file9.ts:178: theta east entry\nsrc/file1.ts:390: green blue eta beta\nsrc/file3.ts:331: ready zeta blue theta\nsrc/file8.ts:154: blue kappa record red red value\nsrc/file3.ts:151: alpha done delta check\nsrc/file3.ts:283: omega value gamma kappa\nsrc/file6.ts:22: delta build iota check gamma red\nsrc/file1.ts:207: blue gamma\nsrc/file5.ts:326: south entry alpha status record\nsrc/file8.ts:89: ready east\nsrc/file9.ts:30: gamma test iota\nsrc/file2.ts:372: epsilon done ready item lambda eta"
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
const fakeImage = (value) => text("[image]");
return await (async (tools, image) => {
text(await tools.qip__blender__execute_blender_code({code:"muvutu vef\ngikili = {'vife': vef.bife.vorudidu, 'vivul': vef.fokuped.vivul.dava, 'zilarag': [{'dava':o.dava,'vena':o.vena} buz o in vef.fokuped.vivul.zilarag][:35]}"}));
image(await tools.read({path:"qut1/vife2.piz",offset:null,limit:null}));
text(await tools.bash({command:"rg -n '^qum |^dadub ' qut1/vife3.py qut1/vife4.py qut1/vife5.py; rg -n 'zibiv|depi.R|dunago' qut6/raz -g '*.js'","tafedov":10}));
text(await tools.read({path:"qut1/vife3.py",offset:155,limit:140}));
text(await tools.read({path:"qut1/vife4.py",offset:590,limit:230}));
})(fake, fakeImage);
