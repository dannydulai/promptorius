function __promptorius_binary_path
    set -l data_dir (set -q XDG_DATA_HOME; and echo "$XDG_DATA_HOME"; or echo "$HOME/.local/share")
    echo "$data_dir/promptorius/__promptorius_output"
end

function __promptorius_ensure_binary
    set -l script (set -q XDG_CONFIG_HOME; and echo "$XDG_CONFIG_HOME"; or echo "$HOME/.config")/promptorius/config
    set -l binary (__promptorius_binary_path)
    set -l compiler (command -v promptorius)

    if not test -f "$binary"; or test "$script" -nt "$binary"; or test "$compiler" -nt "$binary"
        promptorius compile "$script" "$binary"
    end
end

function __promptorius_keymap
    switch "$fish_key_bindings"
        case fish_hybrid_key_bindings fish_vi_key_bindings
            switch "$fish_bind_mode"
                case default
                    echo vicmd
                case visual
                    echo visual
                case '*'
                    echo viins
            end
        case '*'
            echo viins
    end
end

function fish_prompt
    set -l exit_code $status
    set -l duration "$CMD_DURATION$cmd_duration"
    set -l job_count (jobs -g 2>/dev/null | count)
    set -l keymap (__promptorius_keymap)

    # Reset exit code on empty enter (no command ran)
    if test -z "$_promptorius_cmd_ran"
        set exit_code 0
        set duration 0
    end
    set -ge _promptorius_cmd_ran

    __promptorius_ensure_binary
    set -l binary (__promptorius_binary_path)

    $binary --var "exit_code:$exit_code" --var "duration:$duration" --var "jobs:$job_count" --var "keymap:$keymap" --var "shell:fish" --var "shlvl:$SHLVL"
end

function fish_right_prompt
    set -l exit_code $__promptorius_last_exit
    set -l duration "$CMD_DURATION$cmd_duration"
    set -l job_count (jobs -g 2>/dev/null | count)
    set -l keymap (__promptorius_keymap)

    if test -z "$__promptorius_last_exit"
        set exit_code 0
        set duration 0
    end

    set -l binary (__promptorius_binary_path)
    $binary --right --var "exit_code:$exit_code" --var "duration:$duration" --var "jobs:$job_count" --var "keymap:$keymap" --var "shell:fish" --var "shlvl:$SHLVL"
end

function __promptorius_preexec --on-event fish_preexec
    set -g _promptorius_cmd_ran 1
end

function __promptorius_postexec --on-event fish_postexec
    set -g __promptorius_last_exit $status
end

# Disable default vi mode prompt — we handle it ourselves
builtin functions -e fish_mode_prompt
