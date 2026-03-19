function __promptorius_cmd_args
    set -l exit_code $argv[1]
    set -l duration $argv[2]
    echo "--cmd :int:exit_code:$exit_code --cmd :int:duration:$duration"
end

function fish_prompt
    set -g __promptorius_last_status $status
    set -l cmd_args (__promptorius_cmd_args $__promptorius_last_status $CMD_DURATION)
    promptorius $cmd_args
end

function fish_right_prompt
    set -l cmd_args (__promptorius_cmd_args $__promptorius_last_status $CMD_DURATION)
    promptorius --right $cmd_args
end
