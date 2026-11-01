// Harvested pi codemode script (0118, D2280), redacted (see REDACTION.md). Tool results
// are synthetic: the sessions do not record them. Each fake tool answers the calls in
// the order the script made them, like the recorded run; a failed call rejects.
const results = {
 "lun__dev_piduvi__artifacts_list": [
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
     "artifacts": [
      {
       "canonical_url": "west build check",
       "capabilities": {
        "change_visibility": false,
        "delete": true,
        "publish_revision": true
       },
       "created_at": "beta",
       "created_by": "south",
       "created_by_lasig": "build kappa lambda done",
       "expires_at": "epsilon blue blue red",
       "id": "beta theta",
       "latest_version": 222,
       "title": "test green west beta",
       "total_bytes": 318,
       "nom_seconds": 125,
       "updated_at": "record blue lambda",
       "url": "epsilon gamma",
       "version": 93,
       "versions": [
        {
         "byte_size": 129,
         "created_at": "north record theta",
         "file_count": 237,
         "note": "kappa eta gamma",
         "version": 390
        }
       ],
       "visibility": "private"
      }
     ],
     "next_cursor": "green kappa build",
     "ok": true,
     "previous_cursor": "check item build"
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
     "artifacts": [
      {
       "canonical_url": "gamma",
       "capabilities": {
        "change_visibility": false,
        "delete": false,
        "publish_revision": false
       },
       "created_at": "alpha build",
       "created_by": "record",
       "created_by_lasig": "beta item gamma done",
       "expires_at": "gamma item value alpha",
       "id": "red epsilon beta",
       "latest_version": 36,
       "title": "status check",
       "total_bytes": 364,
       "nom_seconds": 272,
       "updated_at": "build lambda beta",
       "url": "done beta ready gamma",
       "version": 479,
       "versions": [
        {
         "byte_size": 118,
         "created_at": "blue blue entry build",
         "file_count": 95,
         "note": "east status east alpha",
         "version": 14
        }
       ],
       "visibility": "organization"
      },
      {
       "canonical_url": "kappa alpha test north",
       "capabilities": {
        "change_visibility": false,
        "delete": false,
        "publish_revision": false
       },
       "created_at": "eta east value",
       "created_by": "north",
       "created_by_lasig": "alpha lambda lambda",
       "expires_at": "omega ready omega ready",
       "id": "build ready",
       "latest_version": 325,
       "title": "red alpha",
       "total_bytes": 441,
       "nom_seconds": 114,
       "updated_at": "record",
       "url": "omega",
       "version": 1,
       "versions": [
        {
         "byte_size": 259,
         "created_at": "south omega alpha",
         "file_count": 366,
         "note": "north kappa alpha omega",
         "version": 399
        }
       ],
       "visibility": "private"
      }
     ],
     "next_cursor": "blue",
     "ok": true,
     "previous_cursor": "east"
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
     "artifacts": [
      {
       "canonical_url": "ready",
       "capabilities": {
        "change_visibility": true,
        "delete": false,
        "publish_revision": true
       },
       "created_at": "eta theta",
       "created_by": "eta build check zeta",
       "created_by_lasig": "green value check beta",
       "expires_at": "status omega",
       "id": "test value entry green",
       "latest_version": 78,
       "title": "south record",
       "total_bytes": 188,
       "nom_seconds": 462,
       "updated_at": "build",
       "url": "alpha alpha",
       "version": 363,
       "versions": [
        {
         "byte_size": 126,
         "created_at": "entry",
         "file_count": 363,
         "note": "blue gamma",
         "version": 154
        },
        {
         "byte_size": 155,
         "created_at": "theta status east theta",
         "file_count": 203,
         "note": "record",
         "version": 203
        }
       ],
       "visibility": "private"
      }
     ],
     "next_cursor": "eta done build omega",
     "ok": true,
     "previous_cursor": "status"
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
const all = new Map();
for (const view of ["recent","mine","organization"]) {
  let cursor;
  do {
    const s = (await tools.lun__dev_piduvi__artifacts_list(cursor ? {view, cursor} : {view})).structuredContent;
    for (const a of s.artifacts) all.set(a.id, a);
    cursor = s.next_cursor;
  } while (cursor);
}
return [...all.values()].sort((a,b)=>b.created_at.localeCompare(a.created_at)).map(a=>`${a.created_at} | ${a.updated_at} | ${a.title} | ${a.visibility} | v${a.latest_version}`).join("\n");
})(fake);
