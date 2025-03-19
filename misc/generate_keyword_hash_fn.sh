#!/bin/bash

gperf -m 100 "$@" <<EOF
%ignore-case
%%
$(cat "${BASH_SOURCE[0]%/*}/keywords.txt")
EOF