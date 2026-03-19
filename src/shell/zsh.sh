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
    local compiler="$(command -v promptorius)"
    _promptorius_binary="${XDG_DATA_HOME:-$HOME/.local/share}/promptorius/__promptorius_output"

    if [[ ! -f "$_promptorius_binary" || "$script" -nt "$_promptorius_binary" || "$compiler" -nt "$_promptorius_binary" ]]; then
        promptorius compile "$script" "$_promptorius_binary"
    fi

    # Store values for re-render on keymap change
    _promptorius_exit_code=$exit_code
    _promptorius_duration=$duration_ms
    _promptorius_jobs=$job_count
    _promptorius_keymap="${KEYMAP:-}"

    promptorius_render
}

promptorius_render() {
    [[ -f "$_promptorius_binary" ]] || return

    local -a vars=(
        --var "exit_code:${_promptorius_exit_code:-0}"
        --var "duration:${_promptorius_duration:-0}"
        --var "jobs:${_promptorius_jobs:-0}"
        --var "keymap:${_promptorius_keymap:-}"
        --var "shell:zsh"
        --var "shlvl:${SHLVL}"
    )
    PROMPT="$($_promptorius_binary "${vars[@]}")"
    RPROMPT="$($_promptorius_binary --right "${vars[@]}")"
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
