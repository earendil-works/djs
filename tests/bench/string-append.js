// String building with += (0061, D1610): single characters, lines into a 2.6 MB output,
// template literals, then reads that need the flat string.
// bench: expect 6670637
let s = "";
for (let i = 0; i < 500_000; i++) s += "x";
let out = "";
const line = "2024-01-15 12:00:00 INFO request handled path=/api/v1/items status=200 ms=12\n";
for (let i = 0; i < 32_000; i++) out += line;
let html = "";
for (let i = 0; i < 60_000; i++) html += `<li id="item-${i}">${i * 2}</li>\n`;
return s.length + out.length + html.length + out.indexOf("zz") + html.lastIndexOf("<li id=\"item-59999\">");
