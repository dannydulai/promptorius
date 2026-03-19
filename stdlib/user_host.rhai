// Show username@hostname (useful for SSH sessions)
let ssh = env("SSH_CONNECTION");

if ssh == "" {
    return ();
}

let user = env("USER");
let host = exec("hostname", ["-s"]);

color("muted") + user + "@" + host + color("")
