#!/bin/bash

shopt -s expand_aliases

#shellcheck source=/home/.bashrc
source ~/.bashrc

docker build -t ardpie .
docker save -o ardpie.tar ardpie
scp_ardpie
