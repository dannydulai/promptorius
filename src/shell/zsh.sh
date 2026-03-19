promptorius_precmd() {
    local exit_code=$?
    local duration=$((EPOCHREALTIME - ${_promptorius_start:-$EPOCHREALTIME}))
    local cmd_args="--cmd :int:exit_code:${exit_code} --cmd :int:duration:${duration}"
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
