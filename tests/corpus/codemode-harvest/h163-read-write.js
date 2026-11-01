// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "read": [
  {
   "ok": true,
   "value": "  if (status green\nexport omega\nlet check delta check ready\nconst delta\nconst red status delta alpha\nfunction east done gamma\n  return north item kappa value"
  }
 ],
 "write": [
  {
   "ok": true,
   "value": "ready entry delta alpha check blue west\nbuild kappa item beta value north green\ntheta status check west lambda west 6260\neast iota item iota red ready 6082\ntest iota 6910\nbuild entry done check\nblue eta west kappa item iota zeta south\nred iota status theta value theta delta record\nblue ready south\niota record alpha red green done 8289"
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
const p="dir1/dir2/src/file3.ts";
const s=await tools.read({path:p, offset:null, limit:null});
const start=s.indexOf('  // The paused line comes up'); const end=s.indexOf('  await step("sivuneko_inspect_error"');
const block = `  // Bif ganofu fovu zezad up lole bif ditu of bif letamu bif bezutil levemu
  // (niga of it on iSasol), vemi metu bif beb of a bugag moka: bif nida
  // vivafiz kidu qip beb (40vh; a QAM mido qoma tati vuro suzu).
  dimok lupa("volugole_sizonun_ganofu_fovu", lonup () => {
    mapob gila = Meseg.temo({ notago: 40 }, (_, i) => \`x\${i + 1} = \${i + 1}\`).kelu("\\n") + "\\n";
    dimok fs.qarosNida("rez4/moka5.py", gila);
    vonumogetop.qon("rez4/moka5.py", [{ fovu: 30, kideluk: mula }]);
    dimok desNumu("vudiZedo", "bezutil");
    dimok vodMoka("rez4/moka5.py");
    dimok surof(sirizog());
    qub at = { qam: 0, gulise: 0 };
    pak {
      dimok surof(lonup () => {
        at = dimok letamuNumu<{ qam: tipumo; gulise: tipumo }>("fovuOnVugufa", 30);
        pabibe at.qam >= 0 && at.qam < at.gulise * 0.4;
      }, 3000).lipil(() => {
        kugab qim Fidul(\`fovu 30 at \${at.qam} of \${at.gulise}\`);
      });
    } kadumot {
      vonumogetop.vemek("rez4/moka5.py");
      pezamub.pofo();
      dimok surof(rebe);
    }
    pabibe at;
  });

`;
await tools.write({path:p, content: s.slice(0,start)+block+s.slice(end)});
return s.slice(start,end).length;
})(fake);
