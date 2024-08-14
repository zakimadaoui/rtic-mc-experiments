#!/bin/bash
cd rtic-core
cargo doc --no-deps
mv target/doc .
cd -
