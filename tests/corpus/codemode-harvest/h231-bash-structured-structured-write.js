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
    "output": "blue item south blue done ready 6399\niota red beta delta delta delta entry\nvalue west theta test\nlambda delta done theta\ndone test lambda zeta record check status\ngreen done ready ready\ncheck check zeta check build 9396\ntest entry 5297\nred delta 2560\nsouth zeta delta green delta east eta value\ntheta entry delta red record epsilon done 4099\ngreen north record blue zeta delta west 8427",
    "truncated": false
   }
  },
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "docs/dir3/dir6/file1.rs\ndocs/dir5/dir1/file2.json\nlib/dir9/file3.rs",
    "truncated": false
   }
  },
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "test/file1.json\ntest/dir5/dir8/file2.js\ndocs/file3.ts\nlib/dir7/dir5/file4.json\nlib/file5.rs\nlib/dir9/dir7/file6.ts\nsrc/dir7/dir2/file7.ts\ndocs/dir7/file8.ts\nsrc/dir4/file9.js\ntest/dir3/file10.rs\nsrc/dir3/file11.js\ndocs/file12.json\ndocs/dir1/file13.md\ntest/file14.json",
    "truncated": false
   }
  },
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "test/file1.ts\ndocs/file2.json\ndocs/file3.ts\nsrc/dir6/dir1/file4.md\nlib/file5.js\nsrc/dir5/dir5/file6.js\nsrc/dir7/dir7/file7.json\ndocs/dir2/dir5/file8.md\nsrc/dir6/dir1/file9.rs\nsrc/file10.ts\nlib/file11.json\ntest/dir4/dir2/file12.md",
    "truncated": false
   }
  },
  {
   "ok": true,
   "value": {
    "exit_code": 0,
    "output": "test/dir1/dir9/file1.md\nlib/dir4/dir9/file2.js\nsrc/dir3/dir7/file3.rs\nlib/file4.rs\ntest/dir9/file5.json",
    "truncated": false
   }
  }
 ],
 "puf__vozaga__tools_imageGeneration_run": [
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
     "images": [
      {
       "dataUrl": "theta gamma east",
       "mediaType": "item north test"
      },
      {
       "dataUrl": "check epsilon beta",
       "mediaType": "gamma theta item check"
      },
      {
       "dataUrl": "check check",
       "mediaType": "omega south"
      }
     ],
     "kind": "vozaga#image_generation",
     "model": "gamma east ready",
     "prompt": "gamma",
     "text": "delta kappa check"
    }
   }
  },
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
     "images": [
      {
       "dataUrl": "west ready",
       "mediaType": "blue red omega lambda"
      },
      {
       "dataUrl": "iota",
       "mediaType": "blue"
      },
      {
       "dataUrl": "blue zeta beta value",
       "mediaType": "eta"
      }
     ],
     "kind": "vozaga#image_generation",
     "model": "item",
     "prompt": "lambda zeta check record",
     "text": "south beta status"
    }
   }
  },
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
     "images": [
      {
       "dataUrl": "theta green green delta",
       "mediaType": "check epsilon status record"
      },
      {
       "dataUrl": "alpha lambda build",
       "mediaType": "test"
      }
     ],
     "kind": "vozaga#image_generation",
     "model": "value east west build",
     "prompt": "south kappa test",
     "text": "test status iota east"
    }
   }
  },
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
     "images": [
      {
       "dataUrl": "east iota south",
       "mediaType": "beta"
      },
      {
       "dataUrl": "alpha zeta item",
       "mediaType": "iota"
      },
      {
       "dataUrl": "kappa test",
       "mediaType": "blue entry gamma"
      }
     ],
     "kind": "vozaga#image_generation",
     "model": "delta",
     "prompt": "omega theta",
     "text": "test lambda check check"
    }
   }
  }
 ],
 "write": [
  {
   "ok": true,
   "value": "record check ready kappa north item kappa\ntest build north omega red beta 4595\ndelta test eta item\nvalue lambda alpha lambda\neast theta kappa check green\nbeta north entry value\nrecord south lambda red 6178\nomega value north\nvalue iota theta done\nstatus omega\ngreen north iota value alpha\nstatus east status check\ngamma value\ngamma green"
  },
  {
   "ok": true,
   "value": "green value west 3266\ngamma value eta iota lambda\nred zeta alpha eta test zeta east\ngreen check done delta\ncheck ready north\nitem iota beta 9083\nstatus lambda kappa delta"
  },
  {
   "ok": true,
   "value": "theta record south red\nalpha item beta theta north ready eta blue\ntest check\nvalue item alpha\ntest beta ready done build gamma epsilon zeta 1515\nbeta entry blue record"
  },
  {
   "ok": true,
   "value": "north check lambda status 3413\nstatus iota gamma red red status\neta eta value\nbuild gamma test green lambda omega theta"
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
return await (async (tools) => {
const prompts = [
  "A zaso fanove zaturi zodiz zukula suzolez a pabo vakufekefez Fugad fumimer on a nepamek forora, rudotap in a nezageg gokat pavovo rupoze, Fugad Novi in qop vutubazira, ferif, mibodalutepeda, desopari",
  "A zaso fanove lemit-zovatul zukula rudotap nesi to a nemeranu zekumi gitone kez (zofolesa) on a risake sufepa fedup in a qibe Fugad neputu, ruvabus dovudipu at a zeguv of mazef on a gitone tisi, lutu novup, mibodalutepeda, desopari",
  "A zaso fanove qike zukula suzolez a pilig rer zatusut qap kig busu a fobeka, fogumen in belud rugo on a qap radot, Fugad lolafumuz kut a neputu in qop vutubazira, mibodalutepeda, desopari",
  "A zaso lizobo pabo nebek kut kapaz zukula baneru up gafa favavi on a rer gefineb busu a kapaz Fugad nugok, on a sufepa bovel nesi to a tupid zanov in an qen Bibalom mefetesiz, duto lutu novup, mibodalutepeda, desopari"
];
await tools.bash({ command: "ragem -p /qet/qel1" });
const results = await Promise.allSettled(prompts.map(async (prompt, i) => {
  const r = await tools.puf__vozaga__tools_imageGeneration_run({ body: { prompt } });
  const res = r.structuredContent ?? r;
  const rul = res.images[0];
  const b64 = rul.dataUrl.split(",")[1];
  await tools.write({ path: `/qet/qel1/qel2${i+1}.b64`, content: b64 });
  const out = await tools.bash({ command: `cd /qet/qel1 && viko64 -d -i k${i+1}.b64 -o k${i+1}.qun && rm k${i+1}.b64 && nafe -s zolale kuza -s zolaleBabasam 82 -Z 900 k${i+1}.qun --ral k${i+1}.raf >/qel3/qel4 && ls -l k${i+1}.raf` });
  return { model: res.model, out: out.output.trim() };
}));
return results.map(r => r.status === "fulfilled" ? r.value : String(r.reason));
})(fake);
