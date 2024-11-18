get_commits() {
    echo $(git rev-list origin/main..HEAD)
}

# Function to reverse a list
reverse_list() {
    input_list="$1"
    reversed=""
    for item in $input_list; do
        reversed="$item $reversed"
    done
    echo "$reversed"
}

commits="$(get_commits)"
commits="$(reverse_list "$commits")"

for commit in $commits; do
    echo "push: $commit"
    git push origin "$commit":main
done
