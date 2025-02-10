#!/bin/bash

set -eu

cd "${BASH_SOURCE%/*}"

# nohup is used to prevent the terminal size from being detected
{
  NO_COLOR=1 nohup cargo run -- --help > ./long_help_no_col.txt
  NO_COLOR=1 nohup cargo run -- -h > ./help_no_col.txt
  NO_COLOR=1 nohup cargo run -- -Chelp > ./config_help_no_col.txt

  CLICOLOR_FORCE=1 nohup cargo run -- --help > ./long_help_col.txt
  CLICOLOR_FORCE=1 nohup cargo run -- -h > ./help_col.txt
  CLICOLOR_FORCE=1 nohup cargo run -- -Chelp > ./config_help_col.txt
} 2>/dev/null