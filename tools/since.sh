#!/bin/bash
# Short summary since the last run: commits, newly blocked tasks, new needs-human items.
# First run: since today 13:00. State: context/notes/since.rev (git-ignored). -n = don't advance.
cd "$(dirname "$0")/.." || exit 1
st=context/notes/since.rev
rev=$(cat "$st" 2>/dev/null) || rev=$(git rev-list -1 --before="today 13:00" HEAD)
cut=(cut -c1-140)  # ponytail: hard line cap keeps it cheap; raise if lines get ambiguous
{
echo "== since $(git log -1 --format='%h %cd' --date=format:'%m-%d %H:%M' "$rev")"
echo "-- changes"
git log --no-merges --reverse --format='%s' "$rev"..HEAD | grep -vE '^(TODO|plan):' | "${cut[@]}"  # task-list edits aren't changes
echo "-- newly blocked"
blk() { grep -o '^- \[~\] \*\*[^*]*\*\*' | cut -d'*' -f3 | sort -u; }
comm -13 <(git show "$rev":plan/TODO.md | blk) <(blk < plan/TODO.md) | while read -r id; do
  grep -m1 "^- \[~\] \*\*$id\*\*" plan/TODO.md | sed "s/^- \\[~\\] \\*\\*[^*]*\\*\\* *//; s/.*BLOCKED:\\? *//" | "${cut[@]}" | sed "s/^/$id: /"
done
echo "-- needs human (new)"
old=$(for f in plan/HUMAN.md plan/TODO.md PLAN.md; do git show "$rev:$f" 2>/dev/null; done)  # items moved between files are not new
grep '^- ' plan/HUMAN.md | cut -c3- | while read -r l; do
  grep -qF -- "${l:0:60}" <<<"$old" || echo "$l" | "${cut[@]}"
done
} | sed '/^[=-][=-] /!s/^/  • /; G'  # bullet items, blank line after every line
[ "$1" = -n ] || git rev-parse HEAD > "$st"
