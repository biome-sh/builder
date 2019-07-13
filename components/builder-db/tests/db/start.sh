#!/bin/bash
#
# Oh biome, how you bring me back to my most hack-worthy roots. I love you for it.
#
# What this does - we trap our own exit, and at exit, we send a SIGINT to all the
# children in our process group - this brings biome down. When we run tests, we
# start this script, and it will take care of setting up the test database on your
# behalf, no matter what.
#
# The gpid stuff below is because we need to track the parent process ID of the
# sudo command that executes us.

service postgresql stop

if [ ! -f /bin/bio ]; then
  useradd -r -U hab
  curl https://raw.githubusercontent.com/biome-sh/biome/master/components/bio/install.sh | bash
fi

mkdir -p /hab/svc/postgresql
cp "$DB_TEST_DIR"/pg_hba.conf /hab/svc/postgresql
cp "$DB_TEST_DIR"/user.toml /hab/svc/postgresql
bio sup run core/postgresql &
bio_pid=$!

read -r sudo_ppid < <(ps -p $$ -o 'ppid=')
read -r original_gpid < <(ps -p "$sudo_ppid" -o 'ppid=')
while true; do
  read -r current_gpid < <(ps -p "$sudo_ppid" -o 'ppid=')
  if [ "$original_gpid" != "$current_gpid" ]; then
    echo "Stopping core/postgresql"
    kill $bio_pid
    exit 0
  fi
  sleep 1
done

