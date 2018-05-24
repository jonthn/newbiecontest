#!/bin/sh

if [ -z "${1}" ]; then
	printf "Require cookie as first argument\n"
	exit 1
fi

# epreuve 134
NUM=$(curl --cookie "SMFCookie89=${1}" -kL https://www.newbiecontest.org/epreuves/prog/prog1.php)
REPONSE=$(echo $NUM | cut -d: -f2 | tr -d " ")
curl --cookie "SMFCookie89=${1}" -kL https://www.newbiecontest.org/epreuves/prog/verifpr1.php?solution=$REPONSE
