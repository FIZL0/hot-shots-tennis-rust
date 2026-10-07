#!/usr/bin/env python3
"""Unattended runner, several tasks at once: like tools/overnight.sh, but keeps HST_SLOTS (3) headless Claude
sessions going, each on its own PLAN.md task in its own git worktree, and merges each finished branch into main.

    tmux new -s hst tools/overnight-parallel.py      # (tools/overnight-parallel-5.sh: 5 at once) progress: context/notes/overnight.log, slot logs beside it

Each session is the normal TUI in its own pane (s1..sN, labelled on its border) of one tmux window "agents" in the
runner's session (the master has its own window): watch or type to any of them. Panes are found by their @hst option, so
join/break/swap them freely. Like overnight.sh, tools/overnight-stop.sh ends a session after HST_IDLE (90) idle seconds; typing
into a finished session cancels that, so /exit it yourself or the runner never merges it.

Picking: open `- [ ] **ID**` lines under ## Tasks, in order, skipping split parents, `(after X)` while X is open,
Stretch, parents with open subtasks (P14 while P14c2 is open), and anything tried already tonight. Two running tasks
never share a file named on their lines, except SHARED (play.rs: nearly every task names it). The first open "Next"
task always leads. Worktrees: ../<repo>-slots/s1..sN, kept between tasks so their target/ stays warm (first build is slow);
context/, replacements/ and the ISO are symlinked in. Each slot N has its own PCSX2 (HST_PCSX2=N: copy
N of the user's config in ../<repo>-slots/pcsx2/sN with its own PINE slot, save states and virtual pad; see
tools/pcsx2-hst.sh); the runner closes it when the slot's session ends.

Merging is the master's job: one long-lived session in window "master" (main checkout) that every finished task
reports to; it merges task/<ID> into main, fixes conflicts, runs the tests and keeps context/notes/master.md for you.
Reports queue in context/notes/master.outbox; each waits until the master is idle, then goes in after a /clear with
the master's instructions, so the session lives on across runs without its context growing. Nothing ends the master:
it outlives a done or stopped run, and the next run adopts it. Restarting the runner adopts the master and the sessions still running in panes s1..sN instead of resetting them. A run that only
hit the usage limit is discarded and its slot sleeps until the reset.
"""
import glob, json, os, re, shlex, subprocess as sp, time, uuid
from datetime import datetime, timedelta

ROOT = os.path.dirname(os.path.dirname(os.path.realpath(__file__)))
WT = ROOT + '-slots'
SLOTS, PAUSE = int(os.environ.get('HST_SLOTS', 3)), int(os.environ.get('HST_PAUSE', 60))
SHARED = {'play.rs'}  # ponytail: files tasks may edit at once; the master resolves the clashes
MAX_HOLD = int(os.environ.get('HST_PCSX2_MAX_HOLD', 1200))  # seconds one agent may hold PCSX2 at a time
NOTES = os.path.join(ROOT, 'context/notes')
TASK = re.compile(r'^- \[ \] \*\*([^*\s]+)\*\*(.*)')
PROMPT = """Parallel unattended run: nobody will answer questions. Do task {id} only — `tools/ctx.py {id}` — following \
PLAN.md's rules and AGENT.md. Other agents are working on other tasks at the same time in other worktrees. You are in a \
git worktree on branch task/{id}: commit here only; never touch the main checkout, merge, rebase or push — the runner \
merges your branch into main. You have your own PCSX2 (HST_PCSX2={n} is set: tools/pcsx2-hst.sh starts copy {n}, \
pine.py and tools/vpad.py talk to it; its save states are a copy, so scratch saves to 8/9 are yours). Run anything that \
drives the game in several steps as one command under `tools/pcsx2.sh <cmd>`; each is cut off after {hold} min, so keep \
runs short (record less, split it). Save slot 5 is the only bot-only game; 3 and 4 have P1 human and sit waiting for \
input unless you drive it with tools/vpad.py in the same tools/pcsx2.sh call. Other agents edit play.rs at the same \
time: keep your play.rs changes local (add functions, systems or new modules; don't move, rename or reformat existing \
code) so the merges stay clean. Close it with `tools/pcsx2-hst.sh stop` \
(never pkill pcsx2-qt: the other agents' copies are running too). When done, tick \
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
            if any(o != m.group(1) and o.startswith(m.group(1)) and o[len(m.group(1))].isdigit() != m.group(1)[-1].isdigit()
                   for o in open_ids):
                continue  # a parent: its open subtasks run instead
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
        if all(tid != b[0] and not (files & b[2]) - SHARED for b in busy):
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


def pane(name, cwd, cmd, env=()):
    """Open cmd in a new pane of window "agents" (the master: its own window "master"; made on first use), tagged
    @hst=name; returns its pid. Slots sit side by side, full height, s1..sN from the left."""
    envs, w = [a for e in env for a in ('-e', e)], 'master' if name == 'master' else 'agents'
    where = ['new-window', '-n', w]
    if w in sp.run(['tmux', 'list-windows', '-F', '#{window_name}'], capture_output=True, text=True).stdout.split():
        here = sorted((int(h[1:]), p) for p, h in (l.split('\t') for l in sp.run(
            ['tmux', 'list-panes', '-t', w, '-F', '#{pane_id}\t#{@hst}'], capture_output=True, text=True).stdout.splitlines())
                      if re.fullmatch(r's\d+', h))
        n = int(name[1:]) if re.fullmatch(r's\d+', name) else 0
        lower = [p for k, p in here if k < n]
        # after the nearest lower slot, else before the lowest one, so the order holds as slots come and go
        where = ['split-window', '-h', '-t', lower[-1]] if lower else ['split-window', '-h', '-b', '-t', here[0][1] if here else w]
    pane_id, pid = sp.run(['tmux', *where, '-d', '-c', cwd, *envs, '-P', '-F', '#{pane_id} #{pane_pid}', cmd],
                       capture_output=True, text=True, check=True).stdout.split()
    sp.run(['tmux', 'set-option', '-p', '-t', pane_id, '@hst', name])
    sp.run(['tmux', 'set-option', '-w', '-t', w, 'pane-border-status', 'top'])
    sp.run(['tmux', 'set-option', '-w', '-t', w, 'pane-border-format', ' #{@hst} '])
    sp.run(['tmux', 'select-layout', '-t', w, 'even-horizontal'])
    return int(pid)


def panes():
    """{name: (pane id, pid)} of the master and slot panes in the runner's session."""
    out = {}
    for l in sp.run(['tmux', 'list-panes', '-s', '-F', '#{pane_id}\t#{pane_pid}\t#{@hst}\t#{window_name}\t#{pane_current_path}'],
                    capture_output=True, text=True).stdout.splitlines():
        pane_id, pid, name, window, path = l.split('\t')
        # ponytail: panes from before @hst were windows; drop these two fallbacks once no such run is left going
        name = name or ('s' + path[len(WT) + 2:] if path.startswith(WT + '/s') else 'master' if window == 'master' else '')
        if name:
            out[name] = (pane_id, int(pid))
    return out


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
    cmd = shlex.join(['claude', PROMPT.format(id=tid, n=n, hold=MAX_HOLD // 60), '--session-id', sid, '--permission-mode', 'bypassPermissions',
                      '--disallowedTools', 'AskUserQuestion', '--settings', json.dumps({'hooks': {'Stop': stop, 'StopFailure': stop}})])
    p = pane(f's{n}', d, cmd, [f'HST_PCSX2_MAX_HOLD={MAX_HOLD}', f'HST_PCSX2={n}'])
    log(f'slot {n}: {tid} ({task[1]}; {", ".join(sorted(task[2])) or "no files listed"})')
    return p, sid, since


MASTER = """You are the master of a parallel unattended run (tools/overnight-parallel.py): nobody will answer \
questions. Up to {slots} agents work on PLAN.md tasks in worktrees ../<repo>-slots/sN, each on branch task/<ID>. The \
runner clears your context and sends you these instructions with one message at a time: `REPORT <ID> slot <n>: \
<session, its commits>` as each one ends. Earlier reports' outcomes are at the end of context/notes/master.md. Handle the \
report in this main checkout:
1. If main has uncommitted changes that aren't yours (the user edits PLAN.md), `git stash` them and pop them back after.
2. `git merge --no-edit task/<ID>` (no commits on it: just note that). Resolve every conflict keeping both sides' \
intent (read both sides' commits and journal; tasks run in parallel, so PLAN.md ticks and new struct fields from both \
belong). Then tools/check.sh; fix what the merge broke, never the task's own work; commit.
3. `git branch -d task/<ID>`. If you can't make it pass: `git merge --abort`, leave the branch and say why.
4. Append one line to context/notes/master.md: <ID>: merged / conflicts fixed (what) / left (why), the task's state \
(ticked, part, blocked) and anything the human must do.
Never push, never touch the worktrees, never do task work yourself. When the runner says `RUN DONE`, finish \
master.md with a short summary for the human at the top."""


def master():
    """The master's tmux pane; started once, adopted by a restarted runner."""
    if 'master' not in panes():
        cmd = shlex.join(['claude', '--permission-mode', 'bypassPermissions', '--disallowedTools', 'AskUserQuestion'])
        pane('master', ROOT, cmd)
        log('master session started (window master)')
        time.sleep(20)  # let the TUI come up before the first report is typed into it


OUTBOX = os.path.join(NOTES, 'master.outbox')


def report(msg):
    """Queue a line for the master; on disk, so a restarted runner still delivers it."""
    open(OUTBOX, 'a').write(msg + '\n')


def queued():
    return open(OUTBOX).read().splitlines() if os.path.exists(OUTBOX) else []


def flush():
    """Once the master is idle: /clear it, then type its instructions and the next queued line."""
    if not (q := queued()):
        return
    m = panes().get('master', ['master'])[0]  # master gone: send-keys fails quietly, as it always did
    screen = sp.run(['tmux', 'capture-pane', '-p', '-t', m], capture_output=True, text=True).stdout
    if re.search(r'…\s\(\d|esc to interrupt', screen):
        return  # ponytail: busy = the TUI's spinner line ("✽ Working… (2m 3s"); breaks if Claude Code redraws it
    for keys in ['/clear', ' '.join(MASTER.format(slots=SLOTS).split('\n')) + ' ' + q[0]]:
        sp.run(['tmux', 'send-keys', '-t', m, '-l', keys])
        sp.run(['tmux', 'send-keys', '-t', m, 'Enter'])
        time.sleep(5)  # /clear finishes, the spinner shows before the next look
    open(OUTBOX, 'w').write(''.join(l + '\n' for l in q[1:]))


def adopt():
    """Sessions a previous runner left going in panes s1..sN: {n: ((pid, sid, since), task)}."""
    out, known = {}, {t[0]: t for t in tasks()}
    for name, (_, pid) in panes().items():
        if not re.fullmatch(r's\d+', name) or int(name[1:]) > SLOTS:
            continue
        n, args = int(name[1:]), open(f'/proc/{pid}/cmdline').read().split('\0')
        tid = git('branch', '--show-current', cwd=slot(n)).stdout.strip().removeprefix('task/')
        if '--session-id' in args or '--resume' in args:
            sid = args[args.index('--session-id' if '--session-id' in args else '--resume') + 1]
            since = os.path.getsize(os.path.join(NOTES, f'slot{n}.log'))
            out[n] = ((int(pid), sid, since), known.get(tid, (tid, '', set())))
            log(f'slot {n}: adopted running {tid}')
    return out


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


def main():
    if not os.environ.get('TMUX'):
        raise SystemExit('run me inside tmux: tmux new -s hst tools/overnight-parallel.py')
    os.makedirs(NOTES, exist_ok=True)
    running, procs, tried, free_at, done = {}, {}, set(), {n: datetime.min for n in range(1, SLOTS + 1)}, False
    log(f'parallel run, {SLOTS} slots')
    master()
    for n, (proc, task) in adopt().items():
        procs[n], running[n] = proc, task
    while True:
        for n in free_at:
            if n in procs and not alive(procs[n][0]):
                (p, sid, since), task = procs.pop(n), running.pop(n)
                log(f'slot {n}: {task[0]} exited')
                # close the slot's PCSX2 copy; waits for its lock, so it never cuts a leftover capture short
                sp.Popen([os.path.join(ROOT, 'tools/pcsx2.sh'), os.path.join(ROOT, 'tools/pcsx2-hst.sh'), 'stop'],
                         env={**os.environ, 'HST_PCSX2': str(n)}, stdout=sp.DEVNULL, stderr=sp.DEVNULL)
                if reset := limit_only(n, sid, since):
                    free_at[n] = reset + timedelta(seconds=PAUSE)
                    log(f'slot {n}: {task[0]} only hit the limit; transcript discarded, slot sleeps until {reset:%H:%M}')
                else:
                    tried.add(task[0])  # one attempt per night: ticked, blocked or failed, a human looks next
                    git('checkout', '-q', '--detach', cwd=slot(n))  # frees task/<ID> for the master to delete
                    commits = git('log', '--format=%s', f'main..task/{task[0]}').stdout.splitlines()
                    report(f'REPORT {task[0]} slot {n}: session {sid} ended; {len(commits)} commits: {" / ".join(commits)[:600]}')
                    log(f'{task[0]}: reported to the master')
                    free_at[n] = datetime.now() + timedelta(seconds=PAUSE)
            if n not in procs and datetime.now() >= free_at[n] and (task := pick(running, tried)):
                procs[n], running[n] = start(n, task), task
        if not done and not procs and not pick(running, tried) and all(datetime.now() >= t for t in free_at.values()):
            done = True
            report('RUN DONE')
            log(f'parallel run done; tried tonight: {", ".join(sorted(tried)) or "none"}')
        flush()
        if done and not queued():
            return  # the master stays up for the next run
        time.sleep(10)


if __name__ == '__main__':
    main()
