$env.PROMPT_COMMAND = {||
    let exit_code = $env.LAST_EXIT_CODE? | default 0
    let duration = $env.CMD_DURATION_MS? | default "0"
    let binary = ([$env.XDG_DATA_HOME? | default ($env.HOME + "/.local/share") "promptorius" "__promptorius_output"] | path join)
    let script = ([$env.XDG_CONFIG_HOME? | default ($env.HOME + "/.config") "promptorius" "config"] | path join)

    if (not ($binary | path exists)) or (($script | path stat).modified > ($binary | path stat).modified) {
        promptorius compile $script $binary
    }

    ^$binary --var $"exit_code:($exit_code)" --var $"duration:($duration)" --var "shell:nu" --var $"shlvl:($env.SHLVL? | default 1)"
}

$env.PROMPT_COMMAND_RIGHT = {||
    let binary = ([$env.XDG_DATA_HOME? | default ($env.HOME + "/.local/share") "promptorius" "__promptorius_output"] | path join)
    ^$binary --right --var "shell:nu" --var $"shlvl:($env.SHLVL? | default 1)"
}
