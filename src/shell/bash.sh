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

    local job_count=$(jobs -r | wc -l | tr -d ' ')

    # Auto-recompile if stale
    local script="${XDG_CONFIG_HOME:-$HOME/.config}/promptorius/config"
    local binary="${XDG_DATA_HOME:-$HOME/.local/share}/promptorius/__promptorius_output"
    local compiler="$(command -v promptorius)"

    if [[ ! -f "$binary" || "$script" -nt "$binary" || "$compiler" -nt "$binary" ]]; then
        promptorius compile "$script" "$binary"
    fi

    PS1="$($binary --var "exit_code:${exit_code}" --var "duration:${duration_ms}" --var "jobs:${job_count}" --var "shell:bash" --var "shlvl:${SHLVL}")"
}

trap '__promptorius_timer_start' DEBUG
PROMPT_COMMAND="__promptorius_prompt_command"
