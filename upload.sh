#!/usr/bin/env bash

set -e -u -o pipefail

readonly DOMAIN=""

if [ -z "$DOMAIN" ]; then
  echo "Unspecified domain"
  exit 1
fi

if [ $# -eq 0 ]; then
  declare FILES="-"
else
  declare FILES=$@
fi

for FILE in $FILES; do
  if [ ! -f "$FILE" ]; then
    echo "$FILE: Missing or invalid file"
    exit 1
  fi
done

for FILE in $FILES; do
  curl --header 'Accept: text/plain' --data-binary @"$FILE" "$DOMAIN"; echo
done