function __promptorius_cmd_args
    set -l exit_code $argv[1]
    set -l duration $argv[2]
    set -l job_count (count (jobs -p))
    echo "--cmd :int:exit_code:$exit_code --cmd :int:duration:$duration --cmd :int:jobs:$job_count"
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
