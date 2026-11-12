#compdef dusk

_dusk() {
    local -a commands opts
    commands=(
        'check:report problems with line numbers'
        'preview:live-reloading preview; actions only print'
        'doctor:layer-shell, blur rule, locker and logind'
        'init:write a commented starter config'
        'help:show help'
        'version:show the version'
    )
    opts=(
        '--config[use PATH]:path:_files'
        '--dry-run[the real menu, but actions only print]'
        '--force[with init, overwrite an existing config]'
        '-h[show help]'
        '--help[show help]'
        '-V[show the version]'
        '--version[show the version]'
    )
    _arguments -s $opts \
        '1:command:->command' \
        '*:config path:_files'
    case $state in
    command) _describe 'command' commands ;;
    esac
}

_dusk "$@"
