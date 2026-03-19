// Show non-zero exit codes on the line before the prompt

let code = exit_code();

if code == 0 {
    return ();
}

color("exitcode") + " Exited w/ " + code + " " + color("") + "\n"
