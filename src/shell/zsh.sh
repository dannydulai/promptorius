promptorius_precmd() {
    local exit_code=$?
    local duration_ms=0
    if [[ -n "$_promptorius_start" ]]; then
        local elapsed=$(( EPOCHREALTIME - _promptorius_start ))
        duration_ms=$(( ${elapsed%.*} * 1000 + 10#${${elapsed#*.}:0:3} ))
    fi
    _promptorius_cmd_args="--cmd :int:exit_code:${exit_code} --cmd :int:duration:${duration_ms}"
    _promptorius_keymap="${KEYMAP:-main}"
    promptorius_render
}

promptorius_render() {
    local cmd_args="${_promptorius_cmd_args} --cmd :str:keymap:${_promptorius_keymap}"
    PROMPT="$(promptorius $cmd_args)"
    RPROMPT="$(promptorius --right $cmd_args)"
    zle reset-prompt 2>/dev/null
}

promptorius_preexec() {
    _promptorius_start=$EPOCHREALTIME
}

promptorius_zle_keymap_select() {
    _promptorius_keymap="${KEYMAP:-main}"
    promptorius_render
}

autoload -Uz add-zsh-hook
add-zsh-hook precmd promptorius_precmd
add-zsh-hook preexec promptorius_preexec

zle -N zle-keymap-select promptorius_zle_keymap_select
