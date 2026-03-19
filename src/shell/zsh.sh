promptorius_precmd() {
    local exit_code=$?
    local duration_ms=0
    if [[ -n "$_promptorius_start" ]]; then
        local elapsed=$(( EPOCHREALTIME - _promptorius_start ))
        duration_ms=$(( ${elapsed%.*} * 1000 + 10#${${elapsed#*.}:0:3} ))
    fi
    local cmd_args="--cmd :int:exit_code:${exit_code} --cmd :int:duration:${duration_ms}"
    PROMPT="$(promptorius $cmd_args)"
    RPROMPT="$(promptorius --right $cmd_args)"
    unset _promptorius_start
}

promptorius_preexec() {
    _promptorius_start=$EPOCHREALTIME
}

autoload -Uz add-zsh-hook
add-zsh-hook precmd promptorius_precmd
add-zsh-hook preexec promptorius_preexec
