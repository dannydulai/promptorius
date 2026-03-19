// Show background job count
let count = env("PROMPTORIUS_JOBS");

if count == "" || count == "0" {
    return ();
}

color("muted") + "✦" + count + color("")
