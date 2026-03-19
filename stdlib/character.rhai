// Prompt character: │ for non-root, # for root
// Repeated $SHLVL times
// Colored by vi keymap: #ff40c0 in vicmd, #666 otherwise
let char = if env("USER") == "root" { "#" } else { "│" };

let shlvl = env("SHLVL");
let count = if shlvl == "" { 1 } else { shlvl.parse_int() };
if count < 1 { count = 1; }

let keymap = if env("PROMPTORIUS_KEYMAP") != "" {
    env("PROMPTORIUS_KEYMAP")
} else {
    // Try --cmd provided keymap
    try { keymap() } catch { "" }
};

let col = if keymap == "vicmd" { "char_vicmd" } else { "char_normal" };

let result = " " + color(col);
for i in 0..count {
    result += char;
}
result + color("") + " "
