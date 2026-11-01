// Date (0038): construction, fields, setters, formatting and parsing with fixed times (local time is UTC on both engines).
const d = new Date(2020, 1, 29, 23, 59, 58, 999);
console.log(d.getTime(), d.getFullYear(), d.getMonth(), d.getDate(), d.getDay(), d.getHours(), d.getMinutes(), d.getSeconds(), d.getMilliseconds());
d.setMonth(12); console.log(d.toISOString()); d.setDate(0); console.log(d.toISOString()); d.setHours(25, 61); console.log(d.toISOString());
d.setFullYear(2021, 1, 29); console.log(d.toISOString()); d.setUTCMilliseconds(1234); console.log(d.toISOString()); console.log(d.setTime(NaN), d.setMonth(1), String(d), d.getFullYear());
const n = new Date(NaN); console.log(n.setFullYear(2000), n.toISOString());
console.log(new Date(2020, 0, 1) - new Date(2019, 0, 1), new Date(0) + 1, `${new Date(86400000)}`, new Date(0) == "Thu Jan 01 1970 00:00:00 GMT+0000", +new Date(5));
console.log(new Date(99, 0).getFullYear(), new Date(Date.UTC(-1, 0)).toISOString(), Date.UTC(275760, 8, 13), Date.UTC(275760, 8, 13, 0, 0, 0, 1), new Date(8.64e15 + 1).getTime());
for (const s of ["2020-02-29T12:34:56.789Z", "2020-02-29", "2020-02-29T12:34", "2020-13-01", "Sat Feb 29 2020 12:34:56 GMT+0000", "Sat, 29 Feb 2020 12:34:56 GMT", "Feb 29 2020", "29 February 2020 10:00 PM EST", "2020/2/29 1:2:3", "12/31/1999 23:59", "+002020-02-29T00:00:00Z", "-000000-01-01T00:00:00Z", "2020-02-30", "T", "", "Thu Jan 01 1970 00:00:00 GMT+0100 (CET)"]) console.log(s, Date.parse(s));
console.log(new Date(new Date(5)).getTime(), new Date("2020-01-01").getTime(), new Date({ valueOf() { return 7; } }).getTime(), new Date(true).getTime());
console.log(Object.prototype.toString.call(new Date()), typeof Date.now(), Date.prototype.toGMTString === Date.prototype.toUTCString, new Date(0).toJSON(), new Date(NaN).toJSON(), new Date(0)[Symbol.toPrimitive]("number"));
try { Date.prototype.getTime.call({}); } catch (e) { console.log(String(e)); }
try { new Date(0)[Symbol.toPrimitive]("bogus"); } catch (e) { console.log(String(e)); }
const stamp = new Date(Date.UTC(2024, 6, 4, 9, 3, 2));
const pad = (n) => String(n).padStart(2, "0");
console.log(`${stamp.getUTCFullYear()}-${pad(stamp.getUTCMonth() + 1)}-${pad(stamp.getUTCDate())} ${pad(stamp.getUTCHours())}:${pad(stamp.getUTCMinutes())}`);
console.log(Math.round((Date.UTC(2024, 11, 25) - Date.UTC(2024, 0, 1)) / 86400000), "days");
console.log(typeof Date.now(), Date.now() > 1.6e12);
