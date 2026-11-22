// The same body, dumped with its text inlined: it loads with no sources.
import { search_issues as search } from "@mcp/github";
import * as linear from "@mcp/linear";
import tracker from "@mcp/linear";
const held = linear.create_issue;
const hits = await search({ q: "imports" });
text(`${hits.length} hits`);
const [made, all] = await Promise.all([held({ title: hits[0].title }), linear.list({ path: "open" })]);
text(made);
return [all, typeof tracker, held === linear.create_issue, Object.keys(linear)];
