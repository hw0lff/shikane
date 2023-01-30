#!/usr/bin/env bash
set -euo pipefail

shikane_dir=$(readlink -f -- "$(dirname "$(readlink -f -- "$0")")"/../)
docs="$shikane_dir/docs"
out="$shikane_dir/build"


buildman() {
    mkdir -p "$out"
    local manpages=("shikane.1" "shikane.5")
    local opts=(
        --standalone
        --from markdown --to man
        --template "$docs/shikane.man.template"
        -V "date=$(date +%F)"
    )

    for page in "${manpages[@]}"; do
        local page_opts=(
            --metadata-file "$docs/$page.man.yml"
            "$docs/$page.man.md"
            -o "$out/$page"
        )
        pandoc "${opts[@]}" "${page_opts[@]}"
        gzip -9 -f "$out/$page"
    done
}

cleanman() {
    rm -rf "$out"
}

usage() {
    cat << EOF
Usage:
    man   - build man pages
    clean - remove man pages
EOF
}


if [[ $# -eq 1 ]]; then
    case $1 in
        clean)
            cleanman
            exit 0;;
        man)
            buildman
            exit 0;;
        *)
            usage
            exit 1;;
    esac
fi

usage
exit 1
