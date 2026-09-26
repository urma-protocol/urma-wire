#!/bin/sh
# Per-file, per-rule tally of lawkeeper violations.
# Usage: scripts/law_report.sh [tally|files|rules|raw]
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
mode=${1:-tally}
report=$(lawkeeper 2>&1 || true)

case "$mode" in
raw) printf '%s\n' "$report" ;;
total) printf '%s\n' "$report" | grep -E '^[0-9]+ violations' ;;
rules)
    printf '%s\n' "$report" | awk '
      match($0, /\[[A-Z]+:[0-9]+\]/) {
        rule = substr($0, RSTART+1, RLENGTH-2)
        n = gsub(/\|/, "|", $0) + 1
        count[rule] += n
      }
      END { for (r in count) printf "%5d  %s\n", count[r], r }
    ' | sort -rn
    ;;
files)
    printf '%s\n' "$report" | awk '
      match($0, /\[[A-Z]+:[0-9]+\]/) { rule = substr($0, RSTART+1, RLENGTH-2); line = $0
        gsub(/^[ ]*\[[A-Z]+:[0-9]+\][ ]*/, "", line)
        n = split(line, parts, /\|/)
        for (i = 1; i <= n; i++) {
          loc = parts[i]
          gsub(/^[ ]+|[ ]+$/, "", loc)
          sub(/:[0-9]+$/, "", loc)
          sub(/ \(whole file\)$/, "", loc)
          if (loc != "") count[loc]++
        }
      }
      END { for (f in count) printf "%5d  %s\n", count[f], f }
    ' | sort -rn
    ;;
tally)
    printf '%s\n' "$report" | awk '
      match($0, /\[[A-Z]+:[0-9]+\]/) { rule = substr($0, RSTART+1, RLENGTH-2); line = $0
        gsub(/^[ ]*\[[A-Z]+:[0-9]+\][ ]*/, "", line)
        n = split(line, parts, /\|/)
        for (i = 1; i <= n; i++) {
          loc = parts[i]
          gsub(/^[ ]+|[ ]+$/, "", loc)
          sub(/:[0-9]+$/, "", loc)
          sub(/ \(whole file\)$/, "", loc)
          if (loc != "") count[loc "\t" rule]++
        }
      }
      END { for (k in count) printf "%5d  %s\n", count[k], k }
    ' | sort -rn
    ;;
*) echo "usage: $0 [tally|files|rules|total|raw]" >&2; exit 1 ;;
esac
