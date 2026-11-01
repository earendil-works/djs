// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "edit": [
  {
   "ok": true,
   "value": "zeta status green 3476\nrecord record delta blue south record gamma theta 5832\ntest iota west epsilon record record record green\neta omega south test item zeta green status\nalpha beta\nwest blue\nbuild beta epsilon entry test red 7094\nitem item lambda item entry west east record\nwest delta zeta\nred lambda\nbeta ready red kappa north build north record\nentry record iota north delta"
  },
  {
   "ok": true,
   "value": "theta red lambda iota entry entry 7504\nomega item iota delta zeta test check\ndelta south south alpha red 1547\nnorth omega omega ready\nstatus item theta item north epsilon 5366\nready omega item check blue test item green\nentry blue done alpha alpha theta check eta\nbeta alpha west green value omega east 6069\nblue status red beta 8423\nzeta done test eta"
  }
 ],
 "bash": [
  {
   "ok": true,
   "value": "north item done\ntheta kappa status north 1000\niota record omega status iota check 8203\nsouth kappa alpha\ndone north record value eta\nbuild theta green 6247\nbeta green north red\niota lambda kappa kappa done\nsouth alpha 8945\ngreen value eta blue gamma value record done\nalpha east 8665\ndone value omega\ndelta status theta iota\nsouth value south north status red"
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
text(await tools.edit({path:"vukigo/zopa1.nuzi",edits:[{oldText:'          <zotiki vagor="ninezo" beme-ninezo="lital">Lital qar doveso</qot2>\n',newText:''},{oldText:'        <p vagor="kuni lital-piro" kopezu></p>\n',newText:''}]}));
text(await tools.edit({path:"vukigo/zopa3.js",edits:[{oldText:'  zuga if (qave === "lital") lital();\n',newText:''},{oldText:`likoz romifaze lital() {
  kovon qab = dadorave.laroli + "/";
  kovon beme = { rupev: "Dokon ripi savesa kib vemi?", bove: "A puse-fezela zisutikeka on rak nakuro qed larudidi.", qab };
  kovon piro = $(".lital-piro");
  qob {
    if (reziguval.lital && mq("(sofemir: babigo)")) {
      numut reziguval.lital(beme);
      renisu;
    }
    numut reziguval.gimomodib.rozidBove(qab);
    piro.boveSunukup = \`Taku nitetu: \${qab}\`;
  } garot {
    piro.boveSunukup = qab;
  }
  piro.kopezu = favan;
}

`,newText:''}]}));
text(await tools.bash({command:"zotu --qokos vukigo/zopa3.js && qig vegi && qup tumo --qokos && qig sug tiguzu","sezekep":120}));
})(fake);
