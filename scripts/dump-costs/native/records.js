// scripts/snapshot-research/cases/records.js: 1500 issue records, a class and Maps.
class Bucket {
  constructor(label) { this.label = label; this.issues = []; this.points = 0; }
  add(issue) { this.issues.push(issue); this.points += issue.points ?? 1; }
  get size() { return this.issues.length; }
}
const issues = await tools.list_issues({ repo: "acme/api", state: "open", limit: 1500 });
const buckets = new Map();
for (const issue of issues) {
  for (const label of issue.labels) {
    if (!buckets.has(label)) buckets.set(label, new Bucket(label));
    buckets.get(label).add(issue);
  }
}
const top = [...buckets.values()].sort((a, b) => b.points - a.points).slice(0, 4);
async function details(bucket) {
  const ids = bucket.issues.slice(0, 5).map((i) => i.number);
  const out = [];
  for (const id of ids) out.push(await tools.issue({ repo: "acme/api", number: id }));
  return { label: bucket.label, size: bucket.size, comments: out.reduce((n, d) => n + d.comments.length, 0) };
}
const result = await Promise.all(top.map(details));
console.log(result);
return result;
