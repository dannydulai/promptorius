// Show abbreviated cwd, replacing $HOME with ~
let dir = cwd();
let home = env("HOME");

if dir.starts_with(home) {
    "~" + dir[home.len()..]
} else {
    dir
}
