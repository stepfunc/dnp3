#!/bin/bash

apt-get update && apt-get -y install clang
curl -LO https://github.com/stepfunc/bindgen-static/raw/refs/heads/exe/bindgen
chmod +x bindgen
mv bindgen /usr/local/bin