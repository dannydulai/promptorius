$env.PROMPT_COMMAND = {||
    let exit_code = $env.LAST_EXIT_CODE? | default 0
    let duration = $env.CMD_DURATION_MS? | default "0"
    promptorius --cmd $":int:exit_code:($exit_code)" --cmd $":int:duration:($duration)"
}

$env.PROMPT_COMMAND_RIGHT = {||
    let exit_code = $env.LAST_EXIT_CODE? | default 0
    let duration = $env.CMD_DURATION_MS? | default "0"
    promptorius --right --cmd $":int:exit_code:($exit_code)" --cmd $":int:duration:($duration)"
}
