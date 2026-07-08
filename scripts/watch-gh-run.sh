#!/usr/bin/env bash
set -Eeuo pipefail

RUN_ID="${1:-}"
TIMEOUT_SECONDS="${TIMEOUT_SECONDS:-900}"
INTERVAL_SECONDS="${INTERVAL_SECONDS:-15}"

usage() {
  cat <<EOF
Usage: scripts/watch-gh-run.sh <run-id>

Environment:
  TIMEOUT_SECONDS=900   Maximum wait time.
  INTERVAL_SECONDS=15   Poll interval.
EOF
}

if [[ -z "$RUN_ID" || "$RUN_ID" == "-h" || "$RUN_ID" == "--help" ]]; then
  usage
  exit 2
fi

if ! command -v gh >/dev/null 2>&1; then
  echo "gh is required" >&2
  exit 127
fi

started_at="$(date +%s)"
last_summary=""

while true; do
  now="$(date +%s)"
  elapsed=$((now - started_at))
  if (( elapsed > TIMEOUT_SECONDS )); then
    echo "Timed out after ${TIMEOUT_SECONDS}s waiting for run ${RUN_ID}" >&2
    gh run view "$RUN_ID" --json status,conclusion,url,jobs \
      --jq '{status, conclusion, url, jobs: [.jobs[] | {name, status, conclusion}]}'
    exit 124
  fi

  summary="$(gh run view "$RUN_ID" --json status,conclusion,jobs --jq '
    {
      status: .status,
      conclusion: .conclusion,
      running: ([.jobs[] | select(.status != "completed") | .name] | join(", ")),
      failed: ([.jobs[] | select(.status == "completed" and .conclusion != "success" and .conclusion != "skipped") | "\(.name):\(.conclusion)"] | join(", "))
    }
  ')"

  if [[ "$summary" != "$last_summary" ]]; then
    printf '%s\n' "$summary"
    last_summary="$summary"
  fi

  status="$(gh run view "$RUN_ID" --json status --jq '.status')"
  conclusion="$(gh run view "$RUN_ID" --json conclusion --jq '.conclusion // ""')"
  if [[ "$status" == "completed" ]]; then
    url="$(gh run view "$RUN_ID" --json url --jq '.url')"
    echo "Run ${RUN_ID} completed: ${conclusion}"
    echo "$url"
    [[ "$conclusion" == "success" ]]
    exit $?
  fi

  sleep "$INTERVAL_SECONDS"
done
