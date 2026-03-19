// Prompt character, colored by last exit code
let char = config.get("character") ?? ">";
let code = exit_code();

if code == 0 {
    color("success") + char + color("")
} else {
    color("error") + char + color("")
}
