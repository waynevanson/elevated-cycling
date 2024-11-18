#!/bin/sh

# Default configuration
DEFAULT_MAX_BATCH_SIZE=$((2 ** 30)) # Default batch size (1 GB)

# Parse arguments
MAX_BATCH_SIZE=$DEFAULT_MAX_BATCH_SIZE
REMOTE=""
BRANCH=""

while [ $# -gt 0 ]; do
    case "$1" in
        -m|--max-batch-size)
            MAX_BATCH_SIZE=$2
            shift 2
            ;;
        -*)
            echo "Unknown option: $1"
            exit 1
            ;;
        *)
            if [ -z "$REMOTE" ]; then
                REMOTE="$1"
            elif [ -z "$BRANCH" ]; then
                BRANCH="$1"
            else
                echo "Unknown argument: $1"
                exit 1
            fi
            shift
            ;;
    esac
done

# Validate arguments
if [ -z "$REMOTE" ] || [ -z "$BRANCH" ]; then
    echo "Usage: $0 [-m|--max-batch-size MAX_SIZE] <remote> <branch>"
    exit 1
fi

# Ensure the branch is correct
git fetch "$REMOTE" "$BRANCH"
if [ $? -ne 0 ]; then
    echo "Error: Unable to fetch branch $BRANCH from $REMOTE."
    exit 1
fi

# Get list of local commits not on the remote
COMMITS=$(git rev-list "$REMOTE/$BRANCH"..HEAD)

if [ -z "$COMMITS" ]; then
    echo "No commits to push."
    exit 0
fi

echo "Commits to push: $(echo "$COMMITS" | wc -l)"
echo "Max batch size: $MAX_BATCH_SIZE bytes"

# Function to calculate the size of a commit
calculate_commit_size() {
    commit=$1
    # Check if the commit has a parent
    if git rev-parse "$commit^" >/dev/null 2>&1; then
        git bundle create ./.tmp/tmp.bundle "$commit^..$commit" > /dev/null 2>&1
    else
        git bundle create ./.tmp/tmp.bundle "$commit" > /dev/null 2>&1
    fi
    size=$(stat -c%s ./.tmp/tmp.bundle)
    rm -f ./.tmp/tmp.bundle
    echo "$size"
}

# Function to push a batch of commits
push_batch() {
    batch=$1
    echo "Pushing batch: $batch"
    git push "$REMOTE" "$batch:$BRANCH"
}

# Batch processing
current_batch=""
current_batch_size=0

for commit in $COMMITS; do
    echo commit: "$commit"
    calculate_commit_size "$commit"
    exit 1
    commit_size=$(calculate_commit_size "$commit")
    current_batch_size=$(expr 0 + "$current_batch_size" + "$commit_size" + 0)

    echo yo "$commit_size" "$current_batch_size" yo

    exit 1

    if [ "$current_batch_size" -gt "$MAX_BATCH_SIZE" ]; then
        if [ -n "$current_batch" ]; then
            push_batch "$current_batch"
        fi
        current_batch_size="$commit_size"
        current_batch="$commit"
    else
        current_batch="$current_batch $commit"
    fi
done

if [ -n "$current_batch" ]; then
    push_batch "$current_batch"
fi

echo "All commits pushed."
