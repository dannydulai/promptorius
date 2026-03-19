__promptorius_timer_start() {
    _promptorius_start=${_promptorius_start:-$SECONDS}
}

__promptorius_prompt_command() {
    local exit_code=$?
    local duration=$(( SECONDS - ${_promptorius_start:-$SECONDS} ))
    unset _promptorius_start
    local cmd_args="--cmd :int:exit_code:${exit_code} --cmd :int:duration:${duration}"
    PS1="$(promptorius $cmd_args)"
}

trap '__promptorius_timer_start' DEBUG
PROMPT_COMMAND="__promptorius_prompt_command"
