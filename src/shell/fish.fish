function __promptorius_ensure_binary
    set -l script (set -q XDG_CONFIG_HOME; and echo "$XDG_CONFIG_HOME"; or echo "$HOME/.config")/promptorius/config
    set -l binary (set -q XDG_DATA_HOME; and echo "$XDG_DATA_HOME"; or echo "$HOME/.local/share")/promptorius/__promptorius_output
    set -l compiler (command -v promptorius)

    if not test -f "$binary"; or test "$script" -nt "$binary"; or test "$compiler" -nt "$binary"
        promptorius compile "$script" "$binary"
    end
end

function fish_prompt
    set -g __promptorius_last_status $status
    set -l job_count (count (jobs -p))
    __promptorius_ensure_binary

    set -l binary (set -q XDG_DATA_HOME; and echo "$XDG_DATA_HOME"; or echo "$HOME/.local/share")/promptorius/__promptorius_output
    $binary --var "exit_code:$__promptorius_last_status" --var "duration:$CMD_DURATION" --var "jobs:$job_count" --var "shell:fish" --var "shlvl:$SHLVL"
end

function fish_right_prompt
    set -l binary (set -q XDG_DATA_HOME; and echo "$XDG_DATA_HOME"; or echo "$HOME/.local/share")/promptorius/__promptorius_output
    $binary --right --var "exit_code:$__promptorius_last_status" --var "duration:$CMD_DURATION" --var "shell:fish" --var "shlvl:$SHLVL"
end
