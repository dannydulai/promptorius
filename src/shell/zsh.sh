zmodload zsh/datetime
zmodload zsh/parameter

promptorius_precmd() {
    local exit_code=$?
    local duration_ms=0

    if [[ -n "$_promptorius_cmd_ran" ]]; then
        if [[ -n "$_promptorius_start" ]]; then
            duration_ms=$(( (EPOCHREALTIME - _promptorius_start) * 1000 ))
            duration_ms=${duration_ms%.*}
        fi
        unset _promptorius_cmd_ran
    else
        exit_code=0
        duration_ms=0
    fi
    unset _promptorius_start

    local job_count=${#jobstates[*]}

    # Auto-recompile if stale
    local script="${XDG_CONFIG_HOME:-$HOME/.config}/promptorius/config"
    local binary="${XDG_DATA_HOME:-$HOME/.local/share}/promptorius/__promptorius_output"
    local compiler="$(command -v promptorius)"

    if [[ ! -f "$binary" || "$script" -nt "$binary" || "$compiler" -nt "$binary" ]]; then
        promptorius compile "$script" "$binary"
    fi

    _promptorius_vars=(
        --var "exit_code:${exit_code}"
        --var "duration:${duration_ms}"
        --var "jobs:${job_count}"
        --var "keymap:${_promptorius_keymap:-}"
        --var "shell:zsh"
        --var "shlvl:${SHLVL}"
    )
    PROMPT="$($binary "${_promptorius_vars[@]}")"
    RPROMPT="$($binary --right "${_promptorius_vars[@]}")"
}

promptorius_render() {
    local binary="${XDG_DATA_HOME:-$HOME/.local/share}/promptorius/__promptorius_output"
    [[ -f "$binary" ]] || return
    _promptorius_vars[-4]="--var"
    _promptorius_vars[-3]="keymap:${_promptorius_keymap:-}"
    PROMPT="$($binary "${_promptorius_vars[@]}")"
    RPROMPT="$($binary --right "${_promptorius_vars[@]}")"
    zle reset-prompt 2>/dev/null
}

promptorius_preexec() {
    _promptorius_start=$EPOCHREALTIME
    _promptorius_cmd_ran=1
}

promptorius_zle_keymap_select() {
    _promptorius_keymap="${KEYMAP:-}"
    promptorius_render
}

autoload -Uz add-zsh-hook
add-zsh-hook precmd promptorius_precmd
add-zsh-hook preexec promptorius_preexec

zle -N zle-keymap-select promptorius_zle_keymap_select
