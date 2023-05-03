#!/usr/bin/env bash
set -euo pipefail

shikane_dir=$(readlink -f -- "$(dirname "$(readlink -f -- "$0")")"/../)
docs="$shikane_dir/docs"
out="$shikane_dir/build"


## Build man pages from markdown files, gzip and write them to $out directory
## $1: shikane version included in the man pages
buildman() {
    local version="$1"
    local manpages=("shikane.1" "shikane.5")
    local opts=(
        --standalone
        --from markdown --to man
        --template "$docs/shikane.man.template"
        -V "date=$(date +%F)"
        -V "footer=shikane $version"
    )

    mkdir -p "$out"
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
Usage: build-docs.sh <COMMAND>

Commands:
  man [VERSION]  - build man pages [explicitly use <VERSION> string]
  clean          - remove man pages
EOF
}


if [[ $# -eq 1 || $# -eq 2 ]]; then
    case $1 in
        clean)
            cleanman
            exit 0;;
        man)
            if [[ $# -eq 2 ]]; then
                buildman "$2"
            else
                version="$(grep -m 1 version Cargo.toml | cut -d\" -f2)"
                buildman "$version"
            fi
            exit 0
            ;;
        *)
            usage
            exit 1;;
    esac
fi

usage
exit 1
