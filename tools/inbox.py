#!/usr/bin/env python3
"""Ticket inbox: the human writes `- [ ] <ticket>` lines in context/inbox.md while an agent works; this hook hands
each new one to the agent once. The agent does it, then ticks it `- [x]` and writes its reply indented below.

Hooked on PostToolUse (picked up after the agent's next tool call) and Stop (an agent about to stop with a
ticket open keeps going instead). Seen tickets: context/notes/inbox.seen.
"""
import json, os, sys

ROOT = os.path.join(os.path.dirname(os.path.realpath(__file__)), '..')
INBOX, SEEN = os.path.join(ROOT, 'context/inbox.md'), os.path.join(ROOT, 'context/notes/inbox.seen')

event = json.load(sys.stdin)
read = lambda p: open(p).read().splitlines() if os.path.exists(p) else []
tickets = [l.strip() for l in read(INBOX) if l.startswith('- [ ]')]
seen = set(read(SEEN))
if event.get('hook_event_name') == 'Stop':
    # ponytail: stop_hook_active guard so an unanswerable ticket can't block stopping forever
    if tickets and not event.get('stop_hook_active'):
        print(json.dumps({'decision': 'block', 'reason': 'Open tickets in context/inbox.md:\n' + '\n'.join(tickets)}))
    sys.exit()
new = [t for t in tickets if t not in seen]
if new:
    os.makedirs(os.path.dirname(SEEN), exist_ok=True)
    open(SEEN, 'a').write(''.join(t + '\n' for t in new))
    print(json.dumps({'hookSpecificOutput': {'hookEventName': 'PostToolUse', 'additionalContext':
        'New ticket(s) from the human in context/inbox.md. Handle them (now if quick or urgent, else after the '
        'current step), then mark each `- [x]` and put your reply indented under it:\n' + '\n'.join(new)}}))
