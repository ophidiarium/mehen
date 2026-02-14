#!/bin/bash

# Stop at the first error
set -e

# Get tree-sitter-grammar
TS_CRATE=`grep $1 Cargo.toml | tr -d ' '`

# Disable/Enable CI flag
RUN_CI="no"

# Temporary main branch Cargo.toml filename
MAIN_CARGO_TOML="main-cargo.toml"

# Download main branch Cargo.toml and save it in a temporary file
wget -LqO - https://raw.githubusercontent.com/ophidiarium/mehen/main/Cargo.toml | tr -d ' ' > $MAIN_CARGO_TOML

# Get the name of the current crate
TS_CRATE_NAME=`echo $TS_CRATE | cut -f1 -d "="`

# Get the crate name from the master branch Cargo.toml
MASTER_TS_CRATE_NAME=`grep $TS_CRATE_NAME $MAIN_CARGO_TOML | head -n 1 | cut -f1 -d "="`

# If the current crate name is not present in master branch, exit the script
if [ -z "$MASTER_TS_CRATE_NAME" ]
then
    exit 0
fi

# Get the same crate from the master branch Cargo.toml
MASTER_TS_CRATE=`grep $TS_CRATE $MAIN_CARGO_TOML | head -n 1`

# If the current crate has been updated, save the crate name
if [ -z "$MASTER_TS_CRATE" ]
then
    # Enable CI flag
    RUN_CI="yes"
    # Name of tree-sitter crate
    TREE_SITTER_CRATE=$TS_CRATE_NAME
fi

# Remove temporary master branch Cargo.toml file
rm -rf $MAIN_CARGO_TOML

# If any crates have been updated, exit the script
if [ "$RUN_CI" = "no" ]; then
    exit 0
fi

# Install json minimal tests
JMT_LINK="https://github.com/Luni-4/json-minimal-tests/releases/download"
JMT_VERSION="0.1.9"
curl -L "$JMT_LINK/v$JMT_VERSION/json-minimal-tests-linux.tar.gz" |
tar xz -C $CARGO_HOME/bin

# Use a test repository (configure your own test repository)
TEST_REPO="${TEST_REPO_PATH:-/cache/test-repo}"

# Compute metrics
./check-grammar-crate.py compute-ci-metrics -p $TEST_REPO -g $TREE_SITTER_CRATE

# Count files in metrics directories
OLD=`ls /tmp/$TREE_SITTER_CRATE-old | wc -l`
NEW=`ls /tmp/$TREE_SITTER_CRATE-new | wc -l`

# Print number of files contained in metrics directories
echo "$TREE_SITTER_CRATE-old: $OLD"
echo "$TREE_SITTER_CRATE-new: $NEW"

# If metrics directories differ in number of files,
# print only the files that are in a directory but not in the other one
if [ $OLD != $NEW ]
then
    ONLY_FILES=`diff -q /tmp/$TREE_SITTER_CRATE-old /tmp/$TREE_SITTER_CRATE-new | grep "Only in"`
    echo "$ONLY_FILES"
fi

# Compare metrics
./check-grammar-crate.py compare-metrics -g $TREE_SITTER_CRATE

# Create artifacts to be uploaded (if there are any)
COMPARE=/tmp/$TREE_SITTER_CRATE-compare
if [ "$(ls -A $COMPARE)" ]; then
    # Maximum number of considered minimal tests for a metric
    MT_THRESHOLD=30

    # Output directory path
    OUTPUT_DIR=/tmp/output-$TREE_SITTER_CRATE

    # Grammar name (removes tree-sitter- prefix)
    GRAMMAR_NAME=`echo $TREE_SITTER_CRATE | cut -c 13-`

    # Split files into distinct directories depending on
    # their metric differences
    ./split-minimal-tests.py -i $COMPARE -o $OUTPUT_DIR -t $MT_THRESHOLD

    tar -czvf /tmp/json-diffs-and-minimal-tests-$GRAMMAR_NAME.tar.gz $COMPARE $OUTPUT_DIR
fi
