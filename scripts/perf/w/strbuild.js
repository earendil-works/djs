let out = "";
for (let i = 0; i < 300_000; i++) out += "line " + i + "\n";
const parts = [];
for (let i = 0; i < 100_000; i++) parts.push(`k${i}=${i * 2}`);
return out.length + parts.join("&").length;
