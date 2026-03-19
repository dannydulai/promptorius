// Show non-zero exit codes
let code = exit_code();

if code == 0 {
    return ();
}

color("error") + code + color("")
