# Requires bash 5.0+
if [[ "${BASH_VERSINFO[0]}" -lt 5 ]]; then
    echo "promptorius: bash 5.0+ required (current: ${BASH_VERSION})" >&2
    return 1
fi

__promptorius_preexec() {
    promptorius time
}

__promptorius_prompt_command() {
    local exit_code=$?
    local duration_ms=0

    if [[ -n "$_promptorius_cmd_ran" ]]; then
        if [[ -n "$_promptorius_start" ]]; then
            local now=$(promptorius time)
            duration_ms=$(( now - _promptorius_start ))
        fi
        unset _promptorius_cmd_ran
    else
        exit_code=0
        duration_ms=0
    fi
    _promptorius_start=""

    # Clear completed background jobs before counting (bash bug workaround)
    jobs &>/dev/null
    local job NUM_JOBS=0
    for job in $(jobs -p); do [[ $job ]] && ((NUM_JOBS++)); done

    # Auto-recompile if stale
    local script="${XDG_CONFIG_HOME:-$HOME/.config}/promptorius/config"
    local binary="${XDG_DATA_HOME:-$HOME/.local/share}/promptorius/__promptorius_output"
    local compiler="$(command -v promptorius)"

    if [[ ! -f "$binary" || "$script" -nt "$binary" || "$compiler" -nt "$binary" ]]; then
        promptorius compile "$script" "$binary"
    fi

    _promptorius_exit_code=$exit_code
    _promptorius_duration=$duration_ms
    _promptorius_jobs=$NUM_JOBS

    __promptorius_render
}

__promptorius_render() {
    local binary="${XDG_DATA_HOME:-$HOME/.local/share}/promptorius/__promptorius_output"
    [[ -f "$binary" ]] || return

    # Detect vi keymap via ble.sh if available
    local keymap="viins"
    if [[ ${BLE_ATTACHED-} ]]; then
        case "${_ble_decode_keymap-}" in
            vi_nmap) keymap="vicmd" ;;
            vi_xmap|vi_smap) keymap="visual" ;;
            *) keymap="viins" ;;
        esac
    fi

    local -a vars=(
        --var "exit_code:${_promptorius_exit_code:-0}"
        --var "duration:${_promptorius_duration:-0}"
        --var "jobs:${_promptorius_jobs:-0}"
        --var "keymap:${keymap}"
        --var "shell:bash"
        --var "shlvl:${SHLVL}"
    )

    local left="$($binary "${vars[@]}")"

    if [[ ${BLE_ATTACHED-} ]]; then
        PS1="$left"
        local nlns=${PS1//[!$'\n']}
        bleopt prompt_rps1="$nlns$($binary --right "${vars[@]}")"
    else
        local right="$($binary --right --columns "${COLUMNS:-80}" "${vars[@]}")"
        PS1="${right}${left}"
    fi
}

if [[ ${BLE_VERSION-} && _ble_version -ge 400 ]]; then
    blehook PREEXEC!='_promptorius_start=$(promptorius time); _promptorius_cmd_ran=1'
    blehook PRECMD!='__promptorius_prompt_command'
    # Re-render prompt on vi mode changes
    bleopt keymap_vi_mode_update_prompt=1
else
    # Use PS0 to capture start time (fires once per command, no pipeline re-trigger)
    PS0='${_promptorius_start:$((_promptorius_start="$(__promptorius_preexec)",_promptorius_cmd_ran=1,0)):0}'"${PS0-}"
    shopt -s checkwinsize
    PROMPT_COMMAND="__promptorius_prompt_command"
fi
