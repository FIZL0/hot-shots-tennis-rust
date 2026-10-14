#!/usr/bin/env python3
"""Unattended runner, several tasks at once: like tools/single.sh, but keeps N (3) Claude
sessions going, each on its own plan/TODO.md task in its own git worktree, and merges each finished branch into main.

    tmux new -s hst tools/parallel.sh [N] [p|d] [f]  # e.g. 5 p; f = keep starting past 90% weekly usage (else it waits for the reset); progress: context/notes/overnight.log, slot logs beside it
    HST_LEAN=0 tools/parallel.sh …                     # agents get full Claude Code (plugins, subagents); default lean

Type `s` + Enter in the runner's pane to stop starting new tasks (running ones finish and merge, then the run ends);
`s` again takes it back. Each session is the normal TUI in its own pane (s1..sN, labelled on its border) of one tmux window "agents" in the
runner's session (the master has its own window): watch or type to any of them. Panes are found by their @hst option, so
join/break/swap them freely. Like single.sh, tools/agent-stop.sh ends a session after HST_IDLE (90) idle seconds; typing
into a finished session cancels that, so /exit it yourself or the runner never merges it.

Picking: open `- [ ] **ID**` lines in plan/TODO.md, in order, skipping split parents, `(after X)` while X is open,
Stretch, parents with open subtasks (P14 while P14c2 is open), and anything tried already tonight. Mode p (priority):
each free slot takes the next such task, exactly in order, files may overlap. In every mode, siblings
(same letters+number: M16a/M16b, P11k/P11l) never run at once. Mode d (different): two running tasks
never share a file named on their lines, except SHARED (play.rs: nearly every task names it); the first open "Next"
task always leads and the rest of "Next" waits for it; the same for any section headed "in this order" (one task
at a time each). `dir/*.rs` on a line counts as every file in that dir. Worktrees: ../<repo>-slots/s1..sN, kept between tasks so their target/ stays warm (first build is slow);
context/, mods/ (+ the old replacements/ link) and the ISO are symlinked in. Each slot N has its own PCSX2 (HST_PCSX2=N: copy
N of the user's config in ../<repo>-slots/pcsx2/sN with its own PINE slot, save states and virtual pad; see
tools/pcsx2-hst.sh); the runner closes it when the slot's session ends.

Merging: the runner merges each finished task/<ID> itself, joining both sides of conflicts in plan/ and journal markdown
and skipping the tests when the merged code is what the agent already tested; only code conflicts and failing tests go
to the master, one long-lived session in window "master" (main checkout) that every finished task
reports to; it merges task/<ID> into main, fixes conflicts, runs the tests and keeps context/notes/master.md for you.
Reports queue in context/notes/master.outbox; each waits until the master is idle, then goes in after a /clear with
the master's instructions, so the session lives on across runs without its context growing. Nothing ends the master:
it outlives a done or stopped run, and the next run adopts it. Restarting the runner adopts the master and the sessions still running in panes s1..sN instead of resetting them. A run that only
hit the usage limit is discarded and its slot sleeps until the reset; one that hit it partway waits in its pane (the stop
hook spares it) and Claude Code continues it at the reset.

Weekly cap: no new task starts once the account's 7-day usage is at WEEKLY_MAX % (read from the same endpoint as /usage,
no tokens spent); running ones finish and the run waits (rechecking every 5 min) until the weekly limit resets.
`f` ignores the cap.
"""
import glob, json, os, re, select, shlex, subprocess as sp, sys, threading, time, uuid
from datetime import datetime, timedelta

ROOT = os.path.dirname(os.path.dirname(os.path.realpath(__file__)))
WT = ROOT + '-slots'
HELP = """\
tools/parallel.sh — run several Claude agents on plan/TODO.md tasks at once (run it inside tmux)

  tools/parallel.sh N [p|d] [f]

  N   number of agents at once, e.g. 5                          default: 3
  p   priority: the next open tasks in plan/TODO.md order,
      even if two of them edit the same files
  d   different files: the first open task, then later tasks    default
      that don't touch the same files
  f   force: keep starting tasks past 90% weekly usage          default: pause until the weekly reset

Examples:
  tmux new -s hst tools/parallel.sh 5 p      5 agents, strict TODO order
  tools/parallel.sh 3                        3 agents on different files
  tools/parallel.sh 5 p f                    all options

HST_LEAN=0 tools/parallel.sh …    agents get full Claude Code (plugins, subagents); default is lean (4 tools, ~17k
                                  vs ~50k tokens per turn). Either way they are normal TUIs: attach, watch, type.

While it runs: type s + Enter to stop starting new tasks (again to undo). Log: context/notes/overnight.log
"""
_args = sys.argv[1:]
if not _args or any(a in ('-h', '--help', 'help') for a in _args):
    raise SystemExit(HELP)
SLOTS = int(next((a for a in _args if a.isdigit()), 3))
MODE = next((a for a in _args if a in ('p', 'd')), 'd')
FORCE = 'f' in _args
if any(not re.fullmatch(r'[pdf]|\d+', a) for a in _args) or SLOTS < 1:
    raise SystemExit(f'unknown option: {" ".join(_args)}\n\n{HELP}')
WEEKLY_MAX = 90  # % of the 7-day limit; at or past it no new task starts (unless f)
PAUSE = int(os.environ.get('HST_PAUSE', 60))
# lean prefix (~17k vs ~50k tokens re-read every turn): 4 tools, no user plugins/MCP; project hooks stay.
# Still the normal TUI (attach and type as ever); HST_LEAN=0 gives agents everything back (plugins, subagents)
LEAN = [] if os.environ.get('HST_LEAN') == '0' else \
    ['--tools', 'Bash,Read,Edit,Write', '--setting-sources', 'project,local', '--strict-mcp-config']
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
(never pkill pcsx2-qt: the other agents' copies are running too). In the task's journal entry, list under 'Not verified / not 1:1' everything you couldn't verify against the original or couldn't match exactly, each with the reason (or 'none'). When done, \
`tools/done.sh {id} "<msg>"`; if stuck, `tools/done.sh {id} "<msg>" --blocked "<why>"` per AGENT.md 'If you get stuck', \
and stop. A usage limit \
is not stuck: commit what's solid but don't mark it `[~]` or stop PCSX2 — the session waits for the reset and continues \
the task."""


_weekly = [-1e9, None]  # checked at, last value


def weekly():
    """7-day usage % from the endpoint /usage reads (OAuth token of the local login; costs no tokens), checked at most
    every 5 min; None if it can't be read (expired token: any claude run refreshes it)."""
    if time.time() - _weekly[0] >= 300:
        import urllib.request
        try:
            tok = json.load(open(os.path.expanduser('~/.claude/.credentials.json')))['claudeAiOauth']['accessToken']
            req = urllib.request.Request('https://api.anthropic.com/api/oauth/usage',
                                         headers={'Authorization': f'Bearer {tok}', 'anthropic-beta': 'oauth-2025-04-20'})
            _weekly[1] = json.load(urllib.request.urlopen(req, timeout=10))['seven_day']['utilization']
        except Exception as e:  # ponytail: unreadable = no cap; the per-session limit handling still applies
            log(f'weekly usage unreadable ({e}); not capping')
            _weekly[1] = None
        _weekly[0] = time.time()
    return _weekly[1]


def desktop():
    """True while this user has a Wayland session (PCSX2, screenshots and --shot need one); after a reboot only
    SDDM's greeter (another user) has one until someone logs in."""
    return bool(glob.glob(f'/run/user/{os.getuid()}/wayland-[0-9]*'))


def capped():
    return not FORCE and (w := weekly()) is not None and w >= WEEKLY_MAX


def git(*a, cwd=ROOT):
    return sp.run(['git', *a], cwd=cwd, capture_output=True, text=True)


def log(msg):
    line = f'=== {datetime.now().isoformat(timespec="seconds")} {msg}'
    print(line, flush=True)
    open(os.path.join(NOTES, 'overnight.log'), 'a').write(line + '\n')


def tasks():
    """Open tasks in plan/TODO.md order: (id, section, files)."""
    lines = git('show', 'main:plan/TODO.md').stdout.splitlines()
    open_ids = {m.group(1) for l in lines if (m := TASK.match(l))}
    out, section = [], ''
    for l in lines:
        if l.startswith('### '):
            section = l[4:]
        elif (m := TASK.match(l)) and not section.startswith('Stretch') and 'split into' not in l:
            if any(o != m.group(1) and o.startswith(m.group(1)) and o[len(m.group(1))].isdigit() != m.group(1)[-1].isdigit()
                   for o in open_ids):
                continue  # a parent: its open subtasks run instead
            after = re.search(r'\(after (\S+?)\)', l)
            if after and after.group(1) in open_ids:
                continue
            files = set(re.findall(r'[\w-]+\.rs', m.group(2).split('|')[0]))
            for d in re.findall(r'([\w-]+)/\*\.rs', m.group(2).split('|')[0]):  # `play/*.rs` = every file in play/
                files |= {os.path.basename(f) for f in glob.glob(f'{ROOT}/crates/*/src/{d}/*.rs')}
            out.append((m.group(1), section, files))
    return out


def family(tid):
    """M16a, M16b, M16 -> M16; P11k4 -> P11: siblings share it and never run side by side, in any mode."""
    return re.match(r'[A-Za-z]*\d*', tid).group()


def pick(running, tried):
    busy, seen = list(running.values()), set()
    for tid, section, files in tasks():
        if all(tid != b[0] for b in busy) and git('rev-list', '-1', f'main..task/{tid}').stdout.strip():
            continue  # finished, its branch waits for a merge: starting it again would reset that branch to main
        if any(family(tid) == family(b[0]) for b in busy):
            continue
        if MODE == 'p':
            if tid not in tried and all(tid != b[0] for b in busy):
                return tid, section, files
            continue
        ordered = section.startswith('Next') or 'in this order' in section
        if tid in tried or (ordered and section in seen):
            continue  # an ordered section (Next, "in this order"): only its first untried task is ever eligible
        if ordered:
            seen.add(section)
        if all(tid != b[0] and not (files & b[2]) - SHARED for b in busy):
            return tid, section, files
    return None


def slot(n):
    d = f'{WT}/s{n}'
    if not os.path.isdir(d):
        os.makedirs(WT, exist_ok=True)
        git('worktree', 'add', '-q', '--detach', d, 'main')
    for src in [os.path.join(ROOT, 'context'), os.path.join(ROOT, 'replacements'), os.path.join(ROOT, 'mods'), *glob.glob(os.path.join(ROOT, '*.iso'))]:
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
    co = git('checkout', '-q', '-B', f'task/{tid}', 'main', cwd=d)
    if co.returncode:  # e.g. another slot still holds task/<ID>: never start an agent on the wrong branch
        raise SystemExit(f'slot {n}: checkout task/{tid} failed: {co.stderr.strip()}')
    sid = str(uuid.uuid4())
    out = open(os.path.join(NOTES, f'slot{n}.log'), 'a')
    since = out.tell()
    out.write(f'\n=== {datetime.now().isoformat(timespec="seconds")} {tid} session {sid}\n')
    out.close()
    stop = [{'hooks': [{'type': 'command', 'command': os.path.join(ROOT, 'tools/agent-stop.sh')}]}]
    brief = sp.run([os.path.join(ROOT, 'tools/ctx.py'), 'brief', tid], cwd=d, capture_output=True, text=True).stdout
    prompt = PROMPT.format(id=tid, n=n, hold=MAX_HOLD // 60) + f' Its brief (tools/ctx.py brief {tid}) follows, so ' \
             f"don't gather it again.\n\n{brief}"
    cmd = shlex.join(['claude', prompt, '--session-id', sid, '--permission-mode', 'bypassPermissions',
                      '--disallowedTools', 'AskUserQuestion', '--model', 'opus', *LEAN, '--settings', json.dumps({'hooks': {'Stop': stop, 'StopFailure': stop}})])
    p = pane(f's{n}', d, cmd, [f'HST_PCSX2_MAX_HOLD={MAX_HOLD}', f'HST_PCSX2={n}', 'HST_BG=1',
                    # each slot builds in its own worktree's target/: cargo names a workspace crate's output by its
                    # repo-relative path, so slots sharing one linked each other's crates. sccache shares the dependency
                    # builds instead; no CARGO_TARGET_DIR, sccache hashes CARGO_* env and would never hit across slots
                    'RUSTC_WRAPPER=sccache', 'SCCACHE_CACHE_SIZE=60G'])
    log(f'slot {n}: {tid} ({task[1]}; {", ".join(sorted(task[2])) or "no files listed"})')
    return p, sid, since


MASTER = """You are the master of a parallel unattended run (tools/parallel.sh): nobody will answer \
questions. Up to {slots} agents work on plan/TODO.md tasks in worktrees ../<repo>-slots/sN, each on branch task/<ID>. The \
runner clears your context and sends you these instructions with one message at a time: `REPORT <ID> slot <n>: \
<session, its commits>` as each one ends. Earlier reports' outcomes are at the end of context/notes/master.md. Handle the \
report in this main checkout:
1. If main has uncommitted changes that aren't yours (the user edits PLAN.md or plan/TODO.md), `git stash push -m "master <ID>"` them and \
`git stash pop` that entry back when you're done, whatever happened (merged, aborted, left); never leave a stash behind.
2. `git merge --no-edit task/<ID>` (no commits on it: just note that). Resolve every conflict keeping both sides' \
intent (read both sides' commits and journal; tasks run in parallel, so plan/TODO.md + DONE.md moves and new struct fields from both \
belong; a branch that still ticks/blocks in PLAN.md's old task list: redo that as `tools/ctx.py tick|block <ID>`). \
Then `tools/ctx.py archive` (moves any [x] the merge brought into plan/TODO.md to plan/DONE.md), tools/check.sh in the foreground (about 2 min; the agent \
already passed it on its branch, so skip it when `git diff --name-only task/<ID> HEAD` lists only .md files; never wait on \
other check.sh processes, the slots run their own); fix what the merge broke, never the task's own work; commit.
3. `git branch -d task/<ID>`. If you can't make it pass: `git merge --abort`, leave the branch and say why.
4. Append one line to context/notes/master.md: <ID>: merged / conflicts fixed (what) / left (why), the task's state \
(ticked, part, blocked) and anything the human must do.
5. Commit every change you made in main, so `git status` shows only the user's popped-back edits (context/ is \
gitignored, so master.md stays uncommitted).
6. If tools/check.sh passed, `git push origin main` (never force; if rejected, `git pull --no-rebase`, re-run \
tools/check.sh, push again, or note why it couldn't).
Never touch the worktrees, never do task work yourself. When the runner says `RUN DONE`, finish \
master.md with a short summary for the human at the top."""


MERGE = threading.Lock()  # one merge into main at a time
SUSPECT = re.compile(r'BLOCKED:.*(pcsx2|capture|unattended|screenshot|not launched|recording)', re.I)


PLAIN = re.compile(r'(plan|research/journal)/.*\.md')  # text files whose conflicts the runner joins itself
ITEM = re.compile(r'- \[(.)\] \*\*([^\s*]+)')  # a top-level task line, as tools/ctx.py


def union(files, tid):
    """Resolve conflicts in plan/journal markdown by keeping both sides (git merge-file --union). Then a task line
    that came out twice keeps the branch's copy for its own task and main's for the rest, and TODO.md loses lines
    whose task DONE.md already has as done (a branch that ticked it, or main that archived it)."""
    for f in sorted(files, key=lambda f: f != 'plan/DONE.md'):  # DONE.md first: TODO.md reads it
        sides = []
        for stage in (2, 1, 3):  # ours, base, theirs
            t = os.path.join(NOTES, f'union.{stage}')
            open(t, 'w').write(git('show', f':{stage}:{f}').stdout)  # no base (both added it): empty
            sides.append(t)
        out = git('merge-file', '-p', '--union', *sides)
        if out.returncode < 0:
            return False
        text = out.stdout
        if f in ('plan/TODO.md', 'plan/DONE.md'):
            done = {m.group(2).rstrip('.') for l in open(os.path.join(ROOT, 'plan/DONE.md'))
                    if (m := ITEM.match(l)) and m.group(1) == 'x'} if f == 'plan/TODO.md' else set()
            out_l, seen, mine = [], {}, git('show', f'task/{tid}:{f}').stdout.splitlines()
            for l in text.split('\n'):
                if m := ITEM.match(l):
                    k = m.group(2).rstrip('.')
                    if k in done:
                        continue
                    if k in seen:
                        if (k == tid) == (l in mine):
                            out_l[seen[k]] = l
                        continue
                    seen[k] = len(out_l)
                out_l.append(l)
            text = '\n'.join(out_l)
        open(os.path.join(ROOT, f), 'w').write(text)
        git('add', f)
    return True


def merge(tid):
    """Merge task/<tid> into main the way the master would, without a model: merge, archive ticks, tests, push.
    Returns None when done, else why it was left for the master (main is then as it was)."""
    # the master's reports go first (back-to-back runner merges starved it), but never wait on it more than 30 min:
    # a master that never takes its line (2026-10-14: idle misread as busy for 4 h) mustn't hold every merge back
    t0 = time.time()
    while queued() and time.time() - t0 < 30 * 60:
        time.sleep(10)
    if queued():
        log(f'{tid}: the master has not taken its queued reports in 30 min; merging without waiting')
    with MERGE:
        # the master is merging in main, or someone left conflicts there (a stash pop): queue behind it; merging
        # over unmerged paths "resolves" theirs and commits that instead of the branch (B207, 2026-10-13)
        t0 = time.time()
        while master_busy() or git('diff', '--name-only', '--diff-filter=U').stdout.strip():
            if time.time() - t0 > 60 * 60:  # hard stop: left for the master instead of stalling the run
                return 'main stayed busy (the master, or unmerged paths) for 60 min'
            time.sleep(10)
        # someone's uncommitted edits in main (PLAN.md, TODO notes): out of the way for the merge, back right after it
        stashed = bool(git('status', '--porcelain', '--untracked-files=no').stdout.strip()) \
            and not git('stash', 'push', '-q', '-m', f'runner {tid}').returncode
        try:
            pre = git('rev-parse', 'HEAD').stdout.strip()
            if git('merge', '--no-edit', f'task/{tid}').returncode:
                files = git('diff', '--name-only', '--diff-filter=U').stdout.split()
                if [f for f in files if not PLAIN.fullmatch(f)] or not union(files, tid):
                    git('merge', '--abort')
                    return f'conflicts in {", ".join(files)}'
                git('commit', '-q', '--no-edit')
                log(f'{tid}: the runner joined both sides of {", ".join(files)}')
            sp.run([os.path.join(ROOT, 'tools/ctx.py'), 'archive'], cwd=ROOT, capture_output=True)
            plan = ['plan/TODO.md', 'plan/DONE.md']  # only these: never sweep someone's edits into the commit (-a did)
            if git('status', '--porcelain', '--', *plan).stdout.strip():
                git('commit', '-qm', f'{tid}: archive ticked tasks into plan/DONE.md', '--', *plan)
        finally:
            if stashed and git('stash', 'pop', '-q').returncode:
                git('reset', '-q', '--hard')  # only the half-applied pop goes; the stash itself stays
                log(f'{tid}: main\'s uncommitted changes clash with the merge: they are kept in git stash "runner {tid}"')
        if git('merge-base', '--is-ancestor', f'task/{tid}', 'HEAD').returncode:
            return 'the merge did not land in main (main changed under it); merge it again'
        merged = git('rev-parse', 'HEAD').stdout.strip()
        # the agent's done.sh passed the tests on its branch: rerun them only if main's code differs from the branch's
        tested = all(f.endswith('.md') for f in git('diff', '--name-only', f'task/{tid}', 'HEAD').stdout.split())
        chk = sp.CompletedProcess([], 0, '', '') if tested else \
            sp.run([os.path.join(ROOT, 'tools/check.sh')], cwd=ROOT, capture_output=True, text=True)
        # ponytail: the net:: tests time out under the slots' load; one rerun of just the failures before it counts
        if chk.returncode and (names := re.findall(r'^---- (\S+) stdout ----', chk.stdout, re.M)):
            again = sp.run([os.path.join(ROOT, 'tools/check.sh'), '--', '--exact', *names],
                           cwd=ROOT, capture_output=True, text=True)
            if not again.returncode:
                log(f'{tid}: {", ".join(names)} failed under load, passed on a rerun')
                chk = again
        inherited = ''
        later = git('rev-list', '--reverse', f'{merged}..HEAD').stdout.split()  # committed on main during the tests
        if chk.returncode:
            git('reset', '-q', '--keep', pre)
            # main already failed that way without the branch (B231's flake, a disc test, main's own broken build):
            # not the branch's fault, so merge it anyway and say so instead of spending a model on it
            # ponytail: name-level match only; a branch that breaks an already-failing test differently slips through
            names = re.findall(r'^---- (\S+) stdout ----', chk.stdout, re.M)
            build = 'BUILD FAILED' in chk.stdout
            if not later and (names or build):
                base = sp.run([os.path.join(ROOT, 'tools/check.sh'), *([] if build else ['--', '--exact', *names])],
                              cwd=ROOT, capture_output=True, text=True)
                still = set(re.findall(r'^---- (\S+) stdout ----', base.stdout, re.M))
                if base.returncode and ('BUILD FAILED' in base.stdout if build else still >= set(names)):
                    git('reset', '-q', '--keep', merged)
                    inherited = 'main\'s build already failed' if build else 'already failing on main: ' + ', '.join(names)
                    log(f'{tid}: {inherited}; merged anyway')
        if chk.returncode and not inherited:  # HEAD is at pre already
            if later and git('cherry-pick', *later).returncode:
                git('cherry-pick', '--abort')
                git('reset', '-q', '--keep', later[-1])  # can't replay them alone: keep them with the merge, master sorts it
                return 'tests fail after the merge, and main got commits meanwhile: ' + ' / '.join(chk.stdout.splitlines()[:4])[:400]
            return 'tests fail after the merge: ' + ' / '.join(chk.stdout.splitlines()[:4])[:400]
        git('branch', '-d', f'task/{tid}')
        pushed = git('push', '-q', 'origin', 'main').returncode == 0
        added = git('diff', pre, 'HEAD', '--', 'plan/TODO.md').stdout
        line = next((l for l in open(os.path.join(ROOT, 'plan/TODO.md')) if f'**{tid}**' in l), '')
        state = 'blocked' if line.startswith('- [~]') else 'part' if line else 'ticked'
        sus = [l[1:].strip()[:200] for l in added.splitlines() if l.startswith('+') and SUSPECT.search(l)]
        note = f'{tid}: merged by the runner, {state}' + ('' if pushed else ', push failed') + \
               (f'; tests fail but {inherited}' if inherited else '') + \
               (f'; SUSPECT BLOCK (PCSX2 runs unattended), recheck: {sus[0]}' if sus else '')
        open(os.path.join(NOTES, 'master.md'), 'a').write(note + '\n')
        log(note)
        return None


def finish(tid, n, sid, commits):
    """After a slot's session: merge its branch here; only what this can't merge goes to the master (a model)."""
    if not commits:
        git('branch', '-D', f'task/{tid}')
        open(os.path.join(NOTES, 'master.md'), 'a').write(f'{tid}: no commits\n')
        return log(f'{tid}: no commits')
    if why := merge(tid):
        report(f'REPORT {tid} slot {n}: session {sid} ended; {len(commits)} commits: {" / ".join(commits)[:600]}. '
               f'The runner could not merge it: {why}')
        log(f'{tid}: left for the master ({why})')


def master():
    """The master's tmux pane; started once, adopted by a restarted runner (only one this code started: an older one
    has no idle hooks, so it's replaced while it holds no report)."""
    mark = os.path.join(NOTES, 'master.pane')
    if (old := panes().get('master')) and (open(mark).read() if os.path.exists(mark) else '') != old[0] \
            and not os.path.exists(LOCK) and not os.path.exists(os.path.join(ROOT, '.git', 'MERGE_HEAD')):
        sp.run(['tmux', 'kill-pane', '-t', old[0]])
        log('master session had no idle hooks (an older runner\'s); replaced')
    if 'master' not in panes():
        # its hooks stamp when a prompt goes in and when a turn ends with no background task left to wake it:
        # master_idle() reads those, like agent-stop.sh
        stamp = lambda f, cond='': [{'hooks': [{'type': 'command', 'command': cond + shlex.join(['touch', os.path.join(NOTES, f)])}]}]
        done = stamp('master.idle', "jq -e '(.background_tasks // []) | length == 0' >/dev/null && ")
        hooks = {'UserPromptSubmit': stamp('master.busy'), 'Stop': done, 'StopFailure': done}
        cmd = shlex.join(['claude', '--model', 'opus', '--permission-mode', 'bypassPermissions', '--disallowedTools', 'AskUserQuestion',
                          '--settings', json.dumps({'hooks': hooks})])
        for f in ('master.busy', 'master.idle'):  # a killed master's last stamp would read busy forever
            if os.path.exists(os.path.join(NOTES, f)):
                os.remove(os.path.join(NOTES, f))
        pane('master', ROOT, cmd)
        open(mark, 'w').write(panes()['master'][0])
        log('master session started (window master)')
        time.sleep(20)  # let the TUI come up before the first report is typed into it


OUTBOX = os.path.join(NOTES, 'master.outbox')


def report(msg):
    """Queue a line for the master; on disk, so a restarted runner still delivers it."""
    open(OUTBOX, 'a').write(msg + '\n')


LOCK = os.path.join(NOTES, 'master.lock')  # "<ID> <master.md size> <time>" while the master handles a report


idle_since = None  # when the master's TUI was first seen idle while it holds the lock


def master_idle():
    """Its last turn ended after its last prompt (the master's own Stop/UserPromptSubmit hooks, `master()`); no
    screen reading: a diff on screen with "come back… (14 s)" in it read as busy for 4 h (2026-10-14).
    A turn that ends with background tasks running doesn't count as idle: they wake it again without a prompt."""
    t = lambda f: os.path.getmtime(os.path.join(NOTES, f)) if os.path.exists(os.path.join(NOTES, f)) else 0
    return t('master.idle') >= t('master.busy')


def master_busy():
    """True while the master is on a report: until it appends `<ID>: …` to master.md and main has no merge going.
    Stale after 90 min (the master died or forgot): logged and freed so merges don't stall forever."""
    try:
        tid, size, since = open(LOCK).read().split()
    except (FileNotFoundError, ValueError):
        return os.path.exists(os.path.join(ROOT, '.git', 'MERGE_HEAD'))  # a merge in main we didn't start (restart)
    md = os.path.join(NOTES, 'master.md')
    new = open(md, 'rb').read()[int(size):].decode(errors='replace') if os.path.exists(md) else ''  # size is in bytes
    global idle_since
    idle_since = (idle_since or time.time()) if master_idle() else None
    # done: its master.md line, or idle 2 min (a report it had nothing to do for, e.g. already merged); never mid-merge
    done = (re.search(rf'^{re.escape(tid)}:', new, re.M) or time.time() - (idle_since or time.time()) >= 120) \
        and not os.path.exists(os.path.join(ROOT, '.git', 'MERGE_HEAD'))
    if not done and time.time() - float(since) < 90 * 60:
        return True
    if not done:
        log(f'{tid}: the master has held main for 90 min without a master.md line; freeing it')
    try:
        os.remove(LOCK)
    except FileNotFoundError:
        pass  # the merge thread and the main loop both saw it done
    return False


def queued():
    return open(OUTBOX).read().splitlines() if os.path.exists(OUTBOX) else []


def flush():
    """Once the master is idle: /clear it, then type its instructions and the next queued line."""
    if not (q := queued()):
        return
    master()  # (re)open it at once, even while a merge holds main
    if master_busy() or not MERGE.acquire(blocking=False):
        return  # one merge into main at a time: the master's report, or the runner's own merge
    try:
        hand(q)
    finally:
        MERGE.release()


def hand(q):
    m = panes().get('master', ['master'])[0]  # master gone: send-keys fails quietly, as it always did
    if not master_idle():
        return
    if not (enter(m, '/clear') and enter(m, ' '.join(MASTER.format(slots=SLOTS).split('\n')) + ' ' + q[0])):
        return  # not taken: the line stays queued for the next look
    open(OUTBOX, 'w').write(''.join(l + '\n' for l in q[1:]))
    global idle_since
    idle_since = None
    if m := re.match(r'REPORT (\S+)', q[0]):
        md = os.path.join(NOTES, 'master.md')
        open(LOCK, 'w').write(f'{m.group(1)} {os.path.getsize(md) if os.path.exists(md) else 0} {time.time()}')


def enter(pane_id, text):
    """Type text into a Claude Code pane and submit it; True once its input box is empty again. Keys sent straight
    after each other got mixed up (a leftover "s" before /clear, a report that came out as /resume), so: clear the
    box (Ctrl-U), paste the text in one bracketed paste, let the TUI take it, then Enter until the box empties."""
    sp.run(['tmux', 'send-keys', '-t', pane_id, 'C-u'])
    sp.run(['tmux', 'set-buffer', '-b', 'hst', text])
    sp.run(['tmux', 'paste-buffer', '-p', '-d', '-b', 'hst', '-t', pane_id])
    for _ in range(5):
        time.sleep(2)
        sp.run(['tmux', 'send-keys', '-t', pane_id, 'Enter'])
        time.sleep(3)
        screen = sp.run(['tmux', 'capture-pane', '-p', '-t', pane_id], capture_output=True, text=True).stdout
        box = [l for l in screen.splitlines() if l.startswith('❯')]
        if box and box[-1].strip() == '❯':
            return True
    return False


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
        raise SystemExit('run me inside tmux: tmux new -s hst tools/parallel.sh [N] [p|d]')
    others = [p for p in sp.run(['pgrep', '-f', r'^(python3 \S*(overnight-parallel|tools/parallel)\.py|bash \S*(overnight|single)\.sh)( |$)'], capture_output=True, text=True).stdout.split()
              if int(p) != os.getpid()]
    if others:  # two parallel runners would share worktrees s1..sN; overnight.sh commits on main under the master
        raise SystemExit(f'another runner is going (pid {", ".join(others)}); wait for it to end')
    os.makedirs(NOTES, exist_ok=True)
    running, procs, tried, free_at, done = {}, {}, set(), {n: datetime.min for n in range(1, SLOTS + 1)}, False
    stopping = False  # `s` typed: start nothing new
    log(f'parallel run, {SLOTS} slots, mode {MODE}, weekly usage {weekly()}%' + (' (cap ignored: f)' if FORCE else f' (cap {WEEKLY_MAX}%)'))
    cap_logged = False
    desk_logged = False
    merging = []  # threads merging finished branches into main
    if queued():
        master()  # reports a previous run left; otherwise the master starts only when a merge needs it
    for n, (proc, task) in adopt().items():
        procs[n], running[n] = proc, task
    for n in free_at:
        if n not in procs and os.path.isdir(slot(n)):
            tid = git('branch', '--show-current', cwd=slot(n)).stdout.strip().removeprefix('task/')
            git('checkout', '-q', '--detach', cwd=slot(n))  # idle slots let go of old task/<ID>s, or start() can't take them
            # a session that ended while no runner watched (B347, 2026-10-14: it exited as the runner restarted)
            # left its branch unmerged: merge it now like any finished slot
            if tid and (commits := git('log', '--format=%s', f'main..task/{tid}').stdout.splitlines()):
                log(f'slot {n}: {tid} ended while no runner was up; merging its {len(commits)} commits')
                tried.add(tid)
                (t := threading.Thread(target=finish, args=(tid, n, '(unknown)', commits))).start()
                merging.append(t)
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
                    git('checkout', '-q', '--detach', cwd=slot(n))  # frees task/<ID> for the merge to delete
                    commits = git('log', '--format=%s', f'main..task/{task[0]}').stdout.splitlines()
                    t = threading.Thread(target=finish, args=(task[0], n, sid, commits))
                    t.start()
                    merging.append(t)
                    free_at[n] = datetime.now() + timedelta(seconds=PAUSE)
            if not stopping and n not in procs and datetime.now() >= free_at[n] and (task := pick(running, tried)):
                if capped():
                    if not cap_logged:
                        log(f'weekly usage {weekly()}% >= {WEEKLY_MAX}%: waiting for the weekly reset (restart with f to go on now)')
                        cap_logged = True
                    continue
                if cap_logged:
                    log(f'weekly usage {weekly()}%: below the cap again, starting tasks')
                    cap_logged = False
                if not desktop():
                    if not desk_logged:
                        log('no desktop session (rebooted? SDDM at its password screen): no new tasks until you log in; '
                            'from a shell: tools/desktop-login.sh')
                        print('\a*** NO DESKTOP: log in, or run tools/desktop-login.sh — tasks wait until then ***', flush=True)
                        desk_logged = True
                    continue
                if desk_logged:
                    log('desktop session up again, starting tasks')
                    desk_logged = False
                procs[n], running[n] = start(n, task), task
        merging = [t for t in merging if t.is_alive()]
        # merges still queued (they wait on each other and the master's reports): RUN DONE only once they're in
        if not done and not procs and not merging and (stopping or not pick(running, tried) and all(datetime.now() >= t for t in free_at.values())):
            done = True
            if 'master' in panes() or queued():
                report('RUN DONE')
            log(f'parallel run done; tried tonight: {", ".join(sorted(tried)) or "none"}')
        flush()
        if done and not queued() and not merging:
            return  # the master stays up for the next run
        line = sys.stdin.readline() if select.select([sys.stdin], [], [], 10)[0] else None
        if line == '':
            time.sleep(10)  # stdin closed: no typing, just wait
        elif line and line.strip().lower() == 's':
            stopping = not stopping
            log('stopping: no new tasks; the running ones finish' if stopping else 'stop taken back: starting tasks again')


if __name__ == '__main__':
    main()
