#!/usr/bin/env python3
"""Compare CPU/RAM of a browser's whole process tree.
usage: scripts/bench.py [--wait 30] -- <browser cmd...>      (URLs are appended by you)
Run once per browser with the SAME urls, e.g.:
  scripts/bench.py -- target/release/strata https://en.wikipedia.org https://github.com https://news.ycombinator.com
  scripts/bench.py -- firefox --no-remote --profile /tmp/ffbench https://en.wikipedia.org https://github.com https://news.ycombinator.com
Close other instances of the browser first. Firefox: use a fresh --profile for a fair default."""
import os, subprocess, sys, time, signal
args = sys.argv[1:]
wait = 30
if args[:1] == ['--wait']:
    wait = int(args[1]); args = args[2:]
if args[:1] == ['--']:
    args = args[1:]
hz = os.sysconf('SC_CLK_TCK')

def tree(root):
    kids = {}
    for p in os.listdir('/proc'):
        if p.isdigit():
            try:
                kids.setdefault(int(open(f'/proc/{p}/stat').read().rsplit(')', 1)[1].split()[1]), []).append(int(p))
            except Exception:
                pass
    out, todo = [], [root]
    while todo:
        p = todo.pop(); out.append(p); todo += kids.get(p, [])
    return out

def sample(root):
    pss = cpu = n = 0
    for p in tree(root):
        try:
            for l in open(f'/proc/{p}/smaps_rollup'):
                if l.startswith('Pss:'): pss += int(l.split()[1])
            f = open(f'/proc/{p}/stat').read().rsplit(')', 1)[1].split()
            cpu += int(f[11]) + int(f[12]); n += 1
        except Exception:
            pass
    return pss / 1024, cpu / hz, n

proc = subprocess.Popen(args, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, preexec_fn=os.setsid)
t0 = time.time()
peak = 0
marks = {}
while time.time() - t0 < wait * 2:
    time.sleep(1)
    el = time.time() - t0
    m, c, n = sample(proc.pid)
    peak = max(peak, m)
    if el >= wait and 'load' not in marks:
        marks['load'] = (m, c, n)
    if el >= wait * 2 - 1 and 'idle' not in marks:
        marks['idle'] = (m, c, n)
        break
lm, lc, ln = marks['load']
im, ic, inn = marks['idle']
print(f"cmd: {' '.join(args)}")
print(f"processes: {inn}   peak RAM: {peak:.0f} MB (PSS, shared pages counted fairly)")
print(f"after {wait}s (pages loaded): RAM {lm:.0f} MB, CPU time used {lc:.1f}s")
print(f"idle window ({wait}s): RAM {im:.0f} MB, CPU {(ic-lc)/wait*100:.1f}% of one core")
os.killpg(proc.pid, signal.SIGTERM)
