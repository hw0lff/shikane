#!/usr/bin/env bash
set -euo pipefail

shikane_dir=$(readlink -f -- "$(dirname "$(readlink -f -- "$0")")"/../)
docs="$shikane_dir/docs"
meta="$docs/meta"
out="$shikane_dir/build"
cargo_version="$(grep -m 1 version "${shikane_dir}"/Cargo.toml | cut -d\" -f2)"
manpages=("shikane.1" "shikane.5" "shikanectl.1")
common_opts=(
    --standalone
    --from markdown
    --metadata-file "$meta/shikane.metadata.yml"
    -M "date=$(date +%F)"
)


## Build man pages from markdown files, gzip and write them to $out/man directory
## $1: shikane version included in the man pages
buildman() {
    local version="$1"
    local out="$out/man"
    local man_opts=(
        "${common_opts[@]}"
        --to man
        --template "$meta/pandoc.man.template"
        -M "footer=shikane $version"
    )

    mkdir -p "$out"
    for page in "${manpages[@]}"; do
        local page_section="${page/#*./}"
        local title="${page/%.*/}"
        local page_opts=(
            "${man_opts[@]}"
            -V section="$page_section"
            -V title="$title"
            "$docs/$page.md"
            -o "$out/$page"
        )
        pandoc "${page_opts[@]}"
        gzip -9 -f "$out/$page"
    done
}

## Build html man pages from markdown files and write them to $out/html directory
## $1: shikane version included in the html man pages
buildhtml() {
    local version="$1"
    local out="$out/html"
    local html_opts=(
        "${common_opts[@]}"
        --embed-resources
        --to html
        --template "$meta/pandoc.html.template"
        --email-obfuscation=javascript
        -M title=shikane
        -M "subtitle=shikane $version"
    )

    mkdir -p "$out"
    for page in "${manpages[@]}"; do
        local page_section="${page/#*./}"
        local title="${page/%.*/}"
        local page_opts=(
            "${html_opts[@]}"
            -V title="$title($page_section)"
            "$docs/$page.md"
            -o "$out/$page.html"
        )
        pandoc "${page_opts[@]}"
    done
    pandoc "${html_opts[@]}" --lua-filter="$meta/links-to-html.lua" "$docs/index.md" -o "$out/index.html"
}


cleanman() {
    rm -rf "$out"
}

usage() {
    cat << EOF
Usage: build-docs.sh <COMMAND>

Commands:
  man [VERSION]  - build man pages [explicitly use <VERSION> string]
  html [VERSION] - build html pages [explicitly use <VERSION> string]
  clean          - remove build artifacts
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
                buildman "$cargo_version"
            fi
            exit 0
            ;;
        html)
            if [[ $# -eq 2 ]]; then
                buildhtml "$2"
            else
                buildhtml "$cargo_version"
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
