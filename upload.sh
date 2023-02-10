#!/usr/bin/env bash

set -e -u -o pipefail

readonly DOMAIN=""   # Dont forget the protocol (HTTP(S)).
readonly USERNAME="" # Leave empty if disabled serverside.
readonly PASSWORD="" # Leave empty if disabled serverside.

if [ -z "$DOMAIN" ]; then
  echo "Unspecified domain" >&2
  exit 1
fi

declare CREDENTIALS=""
if [ -n "$USERNAME" -a -n "$PASSWORD" ]; then
  CREDENTIALS="-u $USERNAME:$PASSWORD"
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

upload() {
  declare FILE="$1"
  shift

  if [ "$FILE" == "-" ]; then
    curl "$@" --data-binary @-; echo
  else
    declare FILENAME="$(tr -dc '\40'-'\176' <<< $(basename $FILE))"
    curl "$@" --request POST --header "X-Filename: $FILENAME" --upload-file "$FILE"; echo
  fi
}

for FILE in $FILES; do
  upload $FILE $CREDENTIALS --header 'Accept: text/plain' --header 'Content-Type:' "$DOMAIN/upload"
done
