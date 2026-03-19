__promptorius_timer_start() {
    _promptorius_start=${_promptorius_start:-$(date +%s%3N)}
    _promptorius_cmd_ran=1
}

__promptorius_prompt_command() {
    local exit_code=$?
    local duration_ms=0

    if [[ -n "$_promptorius_cmd_ran" ]]; then
        if [[ -n "$_promptorius_start" ]]; then
            local now=$(date +%s%3N)
            duration_ms=$(( now - _promptorius_start ))
        fi
        unset _promptorius_cmd_ran
    else
        exit_code=0
        duration_ms=0
    fi
    unset _promptorius_start

    PS1="$(promptorius --cmd ":str:shell:bash" --cmd ":int:exit_code:${exit_code}" --cmd ":int:duration:${duration_ms}")"
}

trap '__promptorius_timer_start' DEBUG
PROMPT_COMMAND="__promptorius_prompt_command"
