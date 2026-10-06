#!/bin/bash

set -u

wsm="./target/wasm32-wasip1/release-wasi/floats2tdigest.wasm"

input1() {
  seq 1 5
}

input2() {
  seq 1 1024
}

run_wasi(){
  cat /dev/stdin |
	  wasmtime \
      run \
      --env ENV_INPUT_MODE=string \
      --env ENV_MAX_SIZE=100 \
      "${wsm}"
}

echo input1
input1 | run_wasi | jq -c
echo

echo input2
input2 | run_wasi | jq -c
echo
