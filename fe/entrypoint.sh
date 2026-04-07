#!/bin/sh
set -e

echo "Running database migrations..."
./node_modules/.bin/prisma migrate deploy
echo "Migrations done."

exec node .output/server/index.mjs
