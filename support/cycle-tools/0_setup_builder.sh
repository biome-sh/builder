#!/bin/bash

set -euo pipefail

echo "Assuming start-builder has been run"

echo "Setting log levels and enabling jobsrv features NEWSCHEDULER and CYCLICBUILDGRAPH"
bio config apply builder-jobsrv.default "$(date +%s)" builder-jobsrv.toml
bio config apply builder-worker.default "$(date +%s)" builder-worker.toml
bio config apply builder-api.default 	"$(date +%s)" builder-api.toml

echo "Generating sql to insert plan connections for the gang" 
./plan_insert.sh > plan_connections.sql

echo "Inserting plan connections into the database" 
psql builder -f plan_connections.sql 


