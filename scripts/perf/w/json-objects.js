// JSON.stringify of many similar objects with some non-Latin-1 text (0224: the wide
// output buffer's spills into the heap after a collection).
// bench: expect 30
const words = ["alpha", "beta", "größe", "naïve", "日本", "user", "data"];
const rows = [];
for (let i = 0; i < 5000; i++) {
  rows.push({ id: i, uuid: (i * 2654435761 >>> 0).toString(16), name: words[i % 7] + " " + words[(i * 3) % 7],
    email: "user" + i + "@example.com", active: i % 2 === 0, score: (i % 1000) / 10, tags: [words[i % 5], words[i % 3]] });
}
let n = 0;
for (let r = 0; r < 30; r++) {
  if (JSON.stringify(rows).length > 0) n++;
}
return n;
