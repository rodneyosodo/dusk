_dusk() {
    local cur prev
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD - 1]}"
    local commands="check preview doctor init help version"
    local opts="--config --dry-run --force -h --help -V --version"
    if [[ "$prev" == "--config" || "$prev" == "check" || "$prev" == "preview" ]]; then
        COMPREPLY=($(compgen -f -- "$cur"))
        return
    fi
    COMPREPLY=($(compgen -W "$commands $opts" -- "$cur"))
}
complete -F _dusk dusk
