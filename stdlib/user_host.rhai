// Show username@hostname (useful for SSH sessions)

let ssh = env("SSH_CONNECTION");

if ssh == "" { return (); }

color("user-host") + env("USER") + "@" + ("HOSTNAME") + color("")
