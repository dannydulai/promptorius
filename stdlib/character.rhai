// Prompt character: │ for non-root, # for root
// Repeated $SHLVL times
// Colored by vi keymap: #ff40c0 in vicmd, #666 otherwise
let char = if env("USER") == "root" { "#" } else { "│" };

let shlvl = env("SHLVL");
let count = if shlvl == "" { 1 } else { shlvl.parse_int() };
if count < 1 { count = 1; }

let result = "";

// Show ҂ if there are background jobs
let count = jobs();

if count > 0 {
    result += color("jobs") + "҂ ";
}

if keymap() == "vicmd" {
    result += color("keymap_vicmd")
} else {
    result += color("keymap_normal")
}

for i in 0..count {
    result += char;
}
result + color("") + " "
