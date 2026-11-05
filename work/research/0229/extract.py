# Extracts the codemode tool-call bodies of the agent sessions in ~/.pi/agent/sessions,
# with only the sizes of their results and the names, argument sizes and durations of the
# tool calls they made, into work/scratch/0229/private/codes.jsonl: gitignored scratch,
# private, never committed or quoted. Caps: 200 bodies per file, 64 KiB per body.
#   python3 work/research/0229/extract.py
import os, json, subprocess
root = os.path.expanduser("~/.pi/agent/sessions")
files = subprocess.run(["rg", "-l", "--no-messages", "-F", '"name":"codemode"', "-g", "*.jsonl", root],
                       capture_output=True, text=True).stdout.split()
dest = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../scratch/0229/private")
os.makedirs(dest, exist_ok=True)
out = open(os.path.join(dest, "codes.jsonl"), "w")
n = 0
for fi, f in enumerate(sorted(files)):
    calls = {}
    per = 0
    with open(f, errors="replace") as fh:
        for line in fh:
            if '"codemode"' not in line and '"toolResult"' not in line: continue
            try: o = json.loads(line)
            except: continue
            m = o.get("message") or {}
            c = m.get("content")
            if m.get("role") == "assistant" and isinstance(c, list):
                for x in c:
                    if isinstance(x, dict) and x.get("type") == "toolCall" and x.get("name") == "codemode":
                        code = (x.get("arguments") or {}).get("code")
                        if isinstance(code, str) and per < 200:
                            calls[x.get("id")] = {"file": fi, "code": code[:65536], "ts": o.get("timestamp")}
                            per += 1
            elif m.get("role") == "toolResult" and m.get("toolCallId") in calls:
                r = calls.pop(m["toolCallId"])
                txt = sum(len(x.get("text", "")) for x in c if isinstance(x, dict)) if isinstance(c, list) else 0
                r["resultLen"] = txt; r["isError"] = bool(m.get("isError"))
                d = m.get("details")
                r["detailKeys"] = sorted(d.keys())[:20] if isinstance(d, dict) else None
                cl = d.get("calls") if isinstance(d, dict) else None
                if isinstance(cl, list):
                    r["calls"] = [{"name": x.get("name"), "argLen": len(x.get("args") or ""), "ms": x.get("durationMs"), "status": x.get("status")} for x in cl[:500] if isinstance(x, dict)]
                r["nResults"] = len(c) if isinstance(c, list) else 0
                r["kinds"] = [x.get("type") for x in c][:20] if isinstance(c, list) else []
                out.write(json.dumps(r) + "\n"); n += 1
print(len(files), "files", n, "codes")
