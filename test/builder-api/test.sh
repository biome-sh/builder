#!/bin/bash

# This script exists to run our integration tests.

set -euo pipefail

# make mocha happy by running from the directory it expects
cd "$(dirname "${BASH_SOURCE[0]}")"

clean_test_artifacts() {
   local sql origins
  origins=( neurosis xmen umbrella )

  # clean origins
  local origins origin_tables
  origin_tables=( origin_integrations origin_project_integrations origin_secrets origin_private_encryption_keys origin_public_encryption_keys origin_members origin_channels origin_invitations origin_packages origin_projects origin_public_keys origin_secret_keys audit_package audit_package_group)
  sql=

  for origin in "${origins[@]}"; do
    sql+="DELETE FROM origin_channel_packages WHERE channel_id IN (SELECT id FROM origin_channels WHERE origin='$origin');"

    for table in "${origin_tables[@]}"; do
      sql+="DELETE FROM $table WHERE origin='$origin';"
    done

    sql+="DELETE FROM origins WHERE name='$origin';"
  done

  psql builder -q -c "$sql"

  # clean users
  local users account_tables
  users=( bobo mystique )
  sql=

  for user in "${users[@]}"; do
    sql+="DELETE FROM accounts WHERE name='$user';"
  done

  psql builder -q -c "$sql"

  # clean jobs
  sql=

  for origin in "${origins[@]}"; do
    sql+="DELETE FROM busy_workers WHERE job_id IN (SELECT id FROM jobs WHERE project_name LIKE '$origin%');"
    sql+="DELETE FROM group_projects WHERE project_name LIKE '$origin%';"
    sql+="DELETE FROM groups WHERE project_name LIKE '$origin%';"
    sql+="DELETE FROM jobs WHERE project_name LIKE '$origin%';"
  done

  psql builder -q -c "$sql"
}

if [ -n "${TRAVIS:-}" ]; then
  pushd "$(git rev-parse --show-toplevel)"
  cp /tmp/builder-github-app.pem .secrets/
  cp .secrets/biome-env{.sample,}
  support/linux/provision.sh
  set +u; eval "$(direnv hook bash)"; set -u
  direnv allow

  # Do what `bio setup` would do
  bio origin key generate "$(id -nu)"
  mkdir -p "$HOME/.hab/etc"
  cat <<EOT > "$HOME/.hab/etc/cli.toml"
origin = "$(id -nu)"
EOT
  mkdir -p "$HOME/.hab/cache/analytics"
  touch "$HOME/.hab/cache/analytics/OPTED_OUT"
  # end bio setup

  cat <<'EOT' >> .studiorc
set -x
set -uo pipefail

HAB_FUNC_TEST=arg-to-sup-run sup-run

until bio sup status; do echo "waiting for bio sup to start"; sleep 1; done

if ! bio sup status; then
  echo "SUPERVISOR FAILED TO START"
  exit 2
fi

start-builder

while ! [ -f "/hab/svc/builder-api/files/builder-github-app.pem" ];
do
    echo "Waiting for builder-github-app.pem"
    ls /hab/svc/builder-api/files
    sleep 10
done

echo "BUILDING BUILDER"
build-builder > /dev/null
echo "BUILDER BUILT build-builder returned $?"

bio pkg install core/node
bio pkg binlink core/node --dest /hab/bin

cd /src/test/builder-api
npm install mocha
bio pkg binlink core/coreutils -d /usr/bin env

while bio sup status | grep --quiet down;
do
  echo "Waiting for services to start..."
  sleep 10
done

while ! [ -f "/hab/svc/builder-api/files/builder-github-app.pem" ];
do
    echo "Waiting for builder-github-app.pem"
    ls /hab/svc/builder-api/files
    sleep 10
done

echo "Starting test run..."
npm run mocha
mstat=$?
echo "Test run exit code: $mstat"

if [ $mstat -ne 0 ]; then
    tail -50 /hab/sup/default/sup.log
fi

exit $mstat
EOT
  HAB_STUDIO_SUP=false bio studio enter
else
  clean_test_artifacts # start with a clean slate

  if ! command -v npm >/dev/null 2>&1; then
    bio pkg install core/node -b
  fi

  if ! [ -f /usr/bin/env ]; then
    bio pkg binlink core/coreutils -d /usr/bin env
  fi

  if ! [ -d node_modules/mocha ]; then
    npm install mocha
  fi

  if npm run mocha; then
    echo "All tests passed, performing DB cleanup"
    clean_test_artifacts
  else
    mocha_exit_code=$?
    echo "Tests failed; skipping cleanup to facilitate investigation"
  fi

  exit ${mocha_exit_code:-0}
fi
