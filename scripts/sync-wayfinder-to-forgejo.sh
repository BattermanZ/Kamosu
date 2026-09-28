#!/usr/bin/env bash
# Mirror Wayfinder issues from GitHub to Forgejo. GitHub remains authoritative.
set -euo pipefail

MODE=dry-run
STATE_FILE="${XDG_STATE_HOME:-$HOME/.local/state}/kamosu/wayfinder-forgejo.json"
GH_REPO="${GH_REPO:-BattermanZ/Kamosu}"
FORGEJO_REPO="${FORGEJO_REPO:-battermanz/Kamosu}"
FORGEJO_API="${FORGEJO_API:-https://forgejo.batterlan.cc/api/v1}"

usage() {
  cat <<'EOF'
Usage: sync-wayfinder-to-forgejo.sh [--dry-run|--apply] [--state PATH]

Copies Wayfinder tickets from GitHub to Forgejo. GitHub is the only editable
source of truth; this command never reads changes back from Forgejo.

The default is --dry-run. --apply requires FORGEJO_TOKEN, a Forgejo token with
repository-read and issue-write access to FORGEJO_REPO.

Optional environment variables: GH_REPO, FORGEJO_REPO, FORGEJO_API,
XDG_STATE_HOME. The local state file maps immutable GitHub node IDs to Forgejo
issue indices and is deliberately kept outside the repository.
EOF
}

while (($#)); do
  case "$1" in
    --dry-run) MODE=dry-run ;;
    --apply) MODE=apply ;;
    --state)
      shift
      STATE_FILE=${1:?--state needs a path}
      ;;
    --help|-h) usage; exit 0 ;;
    *) printf 'Unknown argument: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
  shift
done

for command in gh jq curl; do
  command -v "$command" >/dev/null || {
    printf 'Missing required command: %s\n' "$command" >&2
    exit 1
  }
done

if [[ $MODE == apply && -z ${FORGEJO_TOKEN:-} ]]; then
  printf '%s\n' 'FORGEJO_TOKEN is required with --apply.' >&2
  exit 1
fi

forgejo() {
  local method=$1 path=$2 data=${3-}
  local -a args=(--fail-with-body --silent --show-error --request "$method"
    --header 'Accept: application/json' "$FORGEJO_API$path")
  if [[ -n ${FORGEJO_TOKEN:-} ]]; then
    args=(--header "Authorization: token $FORGEJO_TOKEN" "${args[@]}")
  fi
  if [[ -n $data ]]; then
    args=(--header 'Content-Type: application/json' --data "$data" "${args[@]}")
  fi
  curl "${args[@]}"
}

state='{"version":1,"issues":{},"comments":{}}'
if [[ -f $STATE_FILE ]]; then
  state=$(jq -ce 'if (.version == 1 and (.issues | type == "object") and (.comments | type == "object")) then . else error("unsupported state file") end' "$STATE_FILE")
fi

# Fetching through gh deliberately uses its existing GitHub authentication.
issues=$(gh api --paginate "repos/$GH_REPO/issues?state=all&per_page=100" \
  | jq -sc '[.[][] | select(any(.labels[]?; .name | startswith("wayfinder:")))]')

if [[ $(jq 'length' <<<"$issues") == 0 ]]; then
  printf '%s\n' "No Wayfinder issues found in $GH_REPO."
  exit 0
fi

if [[ $MODE == apply ]]; then
  label_catalog=$(forgejo GET "/repos/$FORGEJO_REPO/labels?limit=100")
  while IFS= read -r label_name; do
    [[ -z $label_name ]] && continue
    if [[ $(jq --arg name "$label_name" 'any(.[]; .name == $name)' <<<"$label_catalog") == true ]]; then
      continue
    fi
    printf 'apply label %s: create on Forgejo\n' "$label_name"
    payload=$(jq -cn --arg name "$label_name" '{name:$name, color:"6f42c1", description:"Mirrored Wayfinder label"}')
    forgejo POST "/repos/$FORGEJO_REPO/labels" "$payload" >/dev/null
  done < <(jq -r '[.[] | .labels[]?.name | select(startswith("wayfinder:"))] | unique[]' <<<"$issues")
  label_catalog=$(forgejo GET "/repos/$FORGEJO_REPO/labels?limit=100")
else
  # Label IDs are only needed for writes. This lets a dry run work without a
  # Forgejo credential and show exactly what it would mirror.
  label_catalog='[]'
fi

map_numbers=$(jq -r '.[] | select(any(.labels[]?; .name == "wayfinder:map")) | .number' <<<"$issues")
parent_by_number='{}'
for map_number in $map_numbers; do
  children=$(gh api graphql -f query="query { repository(owner: \"${GH_REPO%%/*}\", name: \"${GH_REPO#*/}\") { issue(number: $map_number) { subIssues(first: 100) { nodes { number } } } } }" \
    --jq '.data.repository.issue.subIssues.nodes[].number')
  while IFS= read -r child_number; do
    [[ -z $child_number ]] || parent_by_number=$(jq --arg child "$child_number" --arg parent "$map_number" '.[$child] = ($parent | tonumber)' <<<"$parent_by_number")
  done <<<"$children"
done

write_state() {
  [[ $MODE == apply ]] || return 0
  mkdir -p "$(dirname "$STATE_FILE")"
  umask 077
  printf '%s\n' "$state" >"$STATE_FILE"
}

mirror_body() {
  local issue=$1 number parent parent_node parent_forgejo original marker
  number=$(jq -r '.number' <<<"$issue")
  original=$(jq -r '.body // ""' <<<"$issue")
  parent=$(jq -r --arg number "$number" '.[$number] // empty' <<<"$parent_by_number")
  marker="<!-- wayfinder-github-issue:$(jq -r '.node_id' <<<"$issue") -->"
  if [[ -n $parent ]]; then
    parent_node=$(jq -r --argjson parent "$parent" '.[] | select(.number == $parent) | .node_id' <<<"$issues")
    parent_forgejo=$(jq -r --arg node "$parent_node" '.issues[$node].forgejo_index // empty' <<<"$state")
    if [[ -z $parent_forgejo && $MODE == apply ]]; then
      printf 'Could not map GitHub parent #%s before mirroring child #%s.\n' "$parent" "$number" >&2
      exit 1
    fi
    printf 'Part of #%s\n\n%s\n\n%s\n' "${parent_forgejo:-PARENT-TO-BE-MAPPED}" "$original" "$marker"
  else
    printf '%s\n\n%s\n' "$original" "$marker"
  fi
}

while IFS= read -r issue; do
  node_id=$(jq -r '.node_id' <<<"$issue")
  gh_number=$(jq -r '.number' <<<"$issue")
  title=$(jq -r '.title' <<<"$issue")
  body=$(mirror_body "$issue")
  state_name=$(jq -r '.state | ascii_downcase' <<<"$issue")
  label_names=$(jq -c '[.labels[].name]' <<<"$issue")
  existing=$(jq -r --arg node "$node_id" '.issues[$node].forgejo_index // empty' <<<"$state")

  if [[ -z $existing ]]; then
    printf '%s #%s: create on Forgejo\n' "$MODE" "$gh_number"
    if [[ $MODE == apply ]]; then
      label_ids=$(jq -c --argjson wanted "$label_names" '[.[] | select(.name as $name | $wanted | index($name)) | .id]' <<<"$label_catalog")
      payload=$(jq -cn --arg title "$title" --arg body "$body" --argjson labels "$label_ids" '{title:$title, body:$body, labels:$labels}')
      created=$(forgejo POST "/repos/$FORGEJO_REPO/issues" "$payload")
      existing=$(jq -r '.number' <<<"$created")
      state=$(jq --arg node "$node_id" --argjson index "$existing" '.issues[$node] = {forgejo_index:$index}' <<<"$state")
      if [[ $state_name == closed ]]; then
        forgejo PATCH "/repos/$FORGEJO_REPO/issues/$existing" '{"state":"closed"}' >/dev/null
      fi
    else
      continue
    fi
  else
    printf '%s #%s -> Forgejo #%s: update\n' "$MODE" "$gh_number" "$existing"
    if [[ $MODE == apply ]]; then
      label_ids=$(jq -c --argjson wanted "$label_names" '[.[] | select(.name as $name | $wanted | index($name)) | .id]' <<<"$label_catalog")
      payload=$(jq -cn --arg title "$title" --arg body "$body" --argjson labels "$label_ids" --arg state "$state_name" '{title:$title, body:$body, labels:$labels, state:$state}')
      forgejo PATCH "/repos/$FORGEJO_REPO/issues/$existing" "$payload" >/dev/null
    fi
  fi
done < <(jq -c 'sort_by(if any(.labels[]?; .name == "wayfinder:map") then 0 else 1 end)[]' <<<"$issues")

write_state
printf '%s\n' "Complete ($MODE). State: $STATE_FILE"
