promptorius_precmd() {
    local exit_code=$?
    local duration_ms=0

    if [[ -n "$_promptorius_cmd_ran" ]]; then
        # A command was executed — show its exit code and duration
        if [[ -n "$_promptorius_start" ]]; then
            local elapsed=$(( EPOCHREALTIME - _promptorius_start ))
            duration_ms=$(( ${elapsed%.*} * 1000 + 10#${${elapsed#*.}:0:3} ))
        fi
        unset _promptorius_cmd_ran
    else
        # No command ran (empty Enter) — suppress exit code
        exit_code=0
        duration_ms=0
    fi
    unset _promptorius_start

    local job_count=${#jobstates[*]}
    _promptorius_cmd_args=(--cmd ":str:shell:zsh" --cmd ":int:exit_code:${exit_code}" --cmd ":int:duration:${duration_ms}" --cmd ":int:jobs:${job_count}")
    _promptorius_keymap="${KEYMAP:-}"
    promptorius_render
}

promptorius_render() {
    local -a cmd_args=("${_promptorius_cmd_args[@]}" --cmd ":str:keymap:${_promptorius_keymap}")
    PROMPT="$(promptorius "${cmd_args[@]}")"
    RPROMPT="$(promptorius --right "${cmd_args[@]}")"
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

zmodload zsh/parameter
autoload -Uz add-zsh-hook
add-zsh-hook precmd promptorius_precmd
add-zsh-hook preexec promptorius_preexec

zle -N zle-keymap-select promptorius_zle_keymap_select
