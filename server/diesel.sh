#!/bin/bash
diesel migration run
diesel print-schema > src/db/schema.rs
# Included editing as otherwise the array structure would include nullable text which is not compatible with Vec<String>
sed -i 's/Array<Nullable<Text>>/Array<Text>/g' src/db/schema.rs
cargo fmt