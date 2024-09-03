#!/bin/bash
cd rtic-core
rm doc -rf
cargo doc --no-deps
mv target/doc .
cd -
