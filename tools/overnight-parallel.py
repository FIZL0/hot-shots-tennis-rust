#!/usr/bin/env python3
"""Unattended runner, several tasks at once: like tools/overnight.sh, but keeps HST_SLOTS (3) headless Claude
sessions going, each on its own PLAN.md task in its own git worktree, and merges each finished branch into main.

    tmux new -s hst tools/overnight-parallel.py      # progress: context/notes/overnight.log, slot logs beside it

Each session is the normal TUI in its own tmux window (s1..sN) of the runner's session: switch to one to watch or
type to it. Like overnight.sh, tools/overnight-stop.sh ends a session after HST_IDLE (90) idle seconds; typing
into a finished session cancels that, so /exit it yourself or the runner never merges it.

Picking: open `- [ ] **ID**` lines under ## Tasks, in order, skipping split parents, `(after X)` while X is open,
Stretch, and anything tried already tonight. Two running tasks never share a PLAN section or a file named on their
lines (most tasks touch play.rs, so that rule is what keeps merges clean). The first open "Next" task always
leads. Worktrees: ../<repo>-slots/s1..sN, kept between tasks so their target/ stays warm (first build is slow);
context/, replacements/ and the ISO are symlinked in. PCSX2 is shared: pine.py and tools/pcsx2.sh take turns.

A merge that conflicts (or a dirty main checkout) leaves branch task/<ID> for you; the log says so. A run that only
hit the usage limit is discarded and its slot sleeps until the reset.
"""
import glob, json, os, re, shlex, subprocess as sp, time, uuid
from datetime import datetime, timedelta

ROOT = os.path.dirname(os.path.dirname(os.path.realpath(__file__)))
WT = ROOT + '-slots'
SLOTS, PAUSE = int(os.environ.get('HST_SLOTS', 3)), int(os.environ.get('HST_PAUSE', 60))
MAX_HOLD = int(os.environ.get('HST_PCSX2_MAX_HOLD', 1200))  # seconds one agent may hold PCSX2 at a time
NOTES = os.path.join(ROOT, 'context/notes')
TASK = re.compile(r'^- \[ \] \*\*([^*\s]+)\*\*(.*)')
PROMPT = """Parallel unattended run: nobody will answer questions. Do task {id} only — `tools/ctx.py {id}` — following \
PLAN.md's rules and AGENT.md. Other agents are working on other tasks at the same time in other worktrees. You are in a \
git worktree on branch task/{id}: commit here only; never touch the main checkout, merge, rebase or push — the runner \
merges your branch into main. PCSX2 is shared: run anything that drives the real game in several steps as one \
command under `tools/pcsx2.sh <cmd>` (it waits its turn); single pine.py tools wait on their own. Each hold is cut off \
after {hold} min so a hung run can't block the others: keep every run shorter (record less, split it). Save slot 5 is the only bot-only game; 3 and 4 have P1 human \
and sit waiting for input unless you drive it with tools/vpad.py in the same tools/pcsx2.sh call. Never close PCSX2 (the other agents may still need it); the runner closes \
it once no agent is left. When done, tick \
{id} in PLAN.md and commit; if stuck, mark it `[~]` per AGENT.md 'If you get stuck', commit, and stop."""


def git(*a, cwd=ROOT):
    return sp.run(['git', *a], cwd=cwd, capture_output=True, text=True)


def log(msg):
    line = f'=== {datetime.now().isoformat(timespec="seconds")} {msg}'
    print(line, flush=True)
    open(os.path.join(NOTES, 'overnight.log'), 'a').write(line + '\n')


def tasks():
    """Open tasks in PLAN.md order: (id, section, files)."""
    lines = git('show', 'main:PLAN.md').stdout.splitlines()
    open_ids = {m.group(1) for l in lines if (m := TASK.match(l))}
    out, section, inside = [], '', False
    for l in lines:
        if l.startswith('## '):
            inside = l.startswith('## Tasks')
        elif l.startswith('### '):
            section = l[4:]
        elif inside and (m := TASK.match(l)) and not section.startswith('Stretch') and 'split into' not in l:
            after = re.search(r'\(after (\S+?)\)', l)
            if after and after.group(1) in open_ids:
                continue
            files = set(re.findall(r'[\w-]+\.rs', m.group(2).split('|')[0]))
            out.append((m.group(1), section, files))
    return out


def pick(running, tried):
    busy, next_seen = list(running.values()), False
    for tid, section, files in tasks():
        if tid in tried or (section.startswith('Next') and next_seen):
            continue  # the user's ordered list: only its first untried task is ever eligible
        next_seen |= section.startswith('Next')
        if all(tid != b[0] and section != b[1] and not files & b[2] for b in busy):
            return tid, section, files
    return None


def slot(n):
    d = f'{WT}/s{n}'
    if not os.path.isdir(d):
        os.makedirs(WT, exist_ok=True)
        git('worktree', 'add', '-q', '--detach', d, 'main')
    for src in [os.path.join(ROOT, 'context'), os.path.join(ROOT, 'replacements'), *glob.glob(os.path.join(ROOT, '*.iso'))]:
        dst = os.path.join(d, os.path.basename(src))
        if os.path.exists(src) and not os.path.lexists(dst):
            os.symlink(src, dst)
    return d


def start(n, task):
    tid = task[0]
    d = slot(n)
    git('reset', '-q', '--hard', cwd=d), git('clean', '-fdq', cwd=d)
    git('checkout', '-q', '-B', f'task/{tid}', 'main', cwd=d)
    sid = str(uuid.uuid4())
    out = open(os.path.join(NOTES, f'slot{n}.log'), 'a')
    since = out.tell()
    out.write(f'\n=== {datetime.now().isoformat(timespec="seconds")} {tid} session {sid}\n')
    out.close()
    stop = [{'hooks': [{'type': 'command', 'command': os.path.join(ROOT, 'tools/overnight-stop.sh')}]}]
    cmd = shlex.join(['claude', PROMPT.format(id=tid, hold=MAX_HOLD // 60), '--session-id', sid, '--permission-mode', 'bypassPermissions',
                      '--disallowedTools', 'AskUserQuestion', '--settings', json.dumps({'hooks': {'Stop': stop, 'StopFailure': stop}})])
    p = int(sp.run(['tmux', 'new-window', '-d', '-n', f's{n}', '-c', d, '-e', f'HST_PCSX2_MAX_HOLD={MAX_HOLD}', '-P', '-F', '#{pane_pid}', cmd],
                   capture_output=True, text=True, check=True).stdout)
    log(f'slot {n}: {tid} ({task[1]}; {", ".join(sorted(task[2])) or "no files listed"})')
    return p, sid, since


def alive(pid):
    try:
        return os.kill(pid, 0) or True
    except ProcessLookupError:
        return False


def limit_only(n, sid, since):
    """If the run did nothing but hit the usage limit: drop its transcript, return the reset time."""
    t = next(iter(glob.glob(os.path.expanduser(f'~/.claude/projects/*/{sid}.jsonl'))), None)
    text = open(t).read() if t else ''
    with open(os.path.join(NOTES, f'slot{n}.log')) as f:
        f.seek(since)
        text += f.read()
    m = re.findall(r'limit · resets (\d+(?::\d+)?[ap]m)', text)
    if not m or '"type":"tool_use"' in text:
        return None
    if t:
        sp.run(['rm', '-rf', t, t[:-6]])
    reset = datetime.strptime(m[-1], '%I:%M%p' if ':' in m[-1] else '%I%p')
    now = datetime.now()
    at = now.replace(hour=reset.hour, minute=reset.minute, second=0)
    if at < now - timedelta(minutes=10):
        at += timedelta(days=1)  # reset is tomorrow
    return max(at, now)


def merge(n, tid):
    branch = f'task/{tid}'
    if git('rev-list', '--count', f'main..{branch}').stdout.strip() == '0':
        return log(f'{tid}: no commits')
    if git('symbolic-ref', '--short', 'HEAD').stdout.strip() != 'main':
        return log(f'{tid}: main checkout is not on main; {branch} left for you to merge')
    r = git('merge', '--no-edit', branch)
    if r.returncode:
        git('merge', '--abort')
        return log(f'{tid}: merge failed, {branch} left for you: {' '.join((r.stdout + r.stderr).split())[:200]}')
    git('checkout', '-q', '--detach', cwd=slot(n))  # a branch checked out in a worktree can't be deleted
    git('branch', '-q', '-d', branch)
    log(f'{tid}: merged into main')


def main():
    if not os.environ.get('TMUX'):
        raise SystemExit('run me inside tmux: tmux new -s hst tools/overnight-parallel.py')
    os.makedirs(NOTES, exist_ok=True)
    if not sp.run(['pgrep', '-f', 'vpad.py serve'], capture_output=True).stdout:
        sp.Popen([os.path.join(ROOT, 'tools/vpad.py'), 'serve'], stdout=open(os.path.join(NOTES, 'overnight.log'), 'a'), stderr=sp.STDOUT)
    running, procs, tried, free_at = {}, {}, set(), {n: datetime.min for n in range(1, SLOTS + 1)}
    log(f'parallel run, {SLOTS} slots')
    while True:
        for n in free_at:
            if n in procs and not alive(procs[n][0]):
                (p, sid, since), task = procs.pop(n), running.pop(n)
                log(f'slot {n}: {task[0]} exited')
                # the last session out closes PCSX2; waits its turn, so it never cuts a capture short
                if not procs:
                    sp.Popen([os.path.join(ROOT, 'tools/pcsx2.sh'), 'pkill', '-x', 'pcsx2-qt'], stdout=sp.DEVNULL, stderr=sp.DEVNULL)
                if reset := limit_only(n, sid, since):
                    free_at[n] = reset + timedelta(seconds=PAUSE)
                    log(f'slot {n}: {task[0]} only hit the limit; transcript discarded, slot sleeps until {reset:%H:%M}')
                else:
                    tried.add(task[0])  # one attempt per night: ticked, blocked or failed, a human looks next
                    merge(n, task[0])
                    free_at[n] = datetime.now() + timedelta(seconds=PAUSE)
            if n not in procs and datetime.now() >= free_at[n] and (task := pick(running, tried)):
                procs[n], running[n] = start(n, task), task
        if not procs and not pick(running, tried) and all(datetime.now() >= t for t in free_at.values()):
            return log(f'parallel run done; tried tonight: {", ".join(sorted(tried)) or "none"}')
        time.sleep(10)


if __name__ == '__main__':
    main()
