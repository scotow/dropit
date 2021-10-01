#!/usr/bin/env bash

set -e -u -o pipefail

readonly DOMAIN=""

if [ -z "$DOMAIN" ]; then
  echo "Unspecified domain" >&2
  exit 1
fi

if [ $# -eq 0 ]; then
  declare FILES="-"
else
  declare FILES=$@
fi

for FILE in $FILES; do
  if [ "$FILE" != "-" -a ! -f "$FILE" ]; then
    echo "$FILE: Missing or invalid file" >&2
    exit 1
  fi
done

for FILE in $FILES; do
  if [ "$FILE" == "-" ]; then
    curl --header 'Accept: text/plain' --data-binary @"$FILE" "$DOMAIN"; echo
  else
    curl --header 'Accept: text/plain' --header "X-Filename: $(tr -dc '\40'-'\176' <<< $(basename $FILE))" --data-binary @"$FILE" "$DOMAIN"; echo
  fi
done
