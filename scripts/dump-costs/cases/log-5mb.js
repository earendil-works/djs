// A 5 MB tool result (a log) kept while the script asks follow-ups about it.
const log = await tools.logs({ service: "api", lines: 45000 });
const errors = log.split("\n").filter((l) => l.includes(" ERROR "));
const codes = [...new Set(errors.map((l) => /code=(\w+)/.exec(l)?.[1]))].slice(0, 3);
const owners = [];
for (const code of codes) owners.push((await tools.lookup({ code })).owner);
return { bytes: log.length, errors: errors.length, owners };
