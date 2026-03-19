function fish_prompt
    set -l exit_code $status
    set -l duration $CMD_DURATION
    promptorius --cmd :int:exit_code:$exit_code --cmd :int:duration:$duration
end

function fish_right_prompt
    set -l exit_code $status
    set -l duration $CMD_DURATION
    promptorius --right --cmd :int:exit_code:$exit_code --cmd :int:duration:$duration
end
