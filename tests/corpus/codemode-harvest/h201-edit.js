// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "edit": [
  {
   "ok": true,
   "value": "theta north item ready eta delta value blue\nbuild delta blue gamma\niota delta red omega\nentry green gamma\ndelta omega south north delta red\nitem value\ntheta check beta alpha\nalpha record east lambda 1077\nready green blue check\nentry blue zeta done test"
  },
  {
   "ok": true,
   "value": "green eta west red epsilon\ncheck blue 8759\nitem south north 606\nomega status red record\ntheta green eta status check alpha status gamma\ndone red red alpha south green\ngamma value check item iota west iota east 7721\nentry delta\nstatus build delta north east iota blue\ndone delta\ngreen eta item beta south"
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
const r = await Promise.all([
tools.edit({path: "zokote/futa1.guni", edits: [
{oldText: "Nirav in my zonu, qurer 30&komu;cm</em>", newText: "Nirav in my zonu, qurer <puti dumu-cm=\"30\">30&komu;cm (12&komu;in)</qen2></em>"},
{oldText: "Mulota, qurer 50&komu;cm</em>", newText: "Mulota, qurer <puti dumu-cm=\"50\">50&komu;cm (20&komu;in)</qen2></em>"},
{oldText: "Visadiz nafotek, qurer 70&komu;cm</em>", newText: "Visadiz nafotek, qurer <puti dumu-cm=\"70\">70&komu;cm (28&komu;in)</qen2></em>"},
]}),
tools.edit({path: "zokote/futa3.js", edits: [{oldText: "ruv (tatun nen of $$(\"[dumu-zimamasa]\")) {", newText: `// Sizugasod qag kilot in cm; fanule qag zeseril zodi bobe sem rerid genak
// ruv US mokagov.
surizanu pelaFanule() {
  ruv (tatun vok of vulupavub.zenuropun?.mugopu ? vulupavub.zenuropun : [vulupavub.sosapeso]) {
    qar {
      tatun vefedu = buv Suvu.Bozere(vok).vefedu;
      if (vefedu) gevitu vefedu === "US";
    } pobod {}
  }
  gevitu semos;
}

{
  tatun fanuleGenak = pelaFanule();
  ruv (tatun el of $$("[dumu-cm]")) {
    tatun cm = Lipofi(el.masumap.cm);
    tatun misogu = \`\${cm}\\u00a0cm\`;
    tatun marirube = \`\${Qobe.sepuf(cm / 2.54)}\\u00a0in\`;
    el.gotiTupevef = fanuleGenak ? \`\${marirube} (\${misogu})\` : \`\${misogu} (\${marirube})\`;
  }
}

ruv (tatun nen of $$("[dumu-zimamasa]")) {`}]})
]);
return r;
})(fake);
