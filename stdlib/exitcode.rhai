let result = "";

// Show non-zero exit codes on the line before the prompt

{
    let code = exit_code();
    if code != 0 {
        result += color("exitcode") + " Exited w/ " + code + " " + color("") + "\n"
    }
}

// Show abbreviated cwd, replacing $HOME with ~
let dir = cwd();
let home = env("HOME");

let display = if dir.starts_with(home) {
    "~" + dir[home.len()..]
} else {
    dir
};

result += color("directory") + display + color("");

result;
