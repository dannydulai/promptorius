// Show last command duration if above threshold

let threshold = config.get("threshold_ms") ?? 2000;
let dur = duration();

if dur < threshold {
    return ();
}

if dur >= 60000 {
    let mins = dur / 60000;
    let secs = (dur % 60000) / 1000;
    color("duration") + mins + "m" + secs + "s" + color("") + " "
} else if dur >= 1000 {
    let secs = dur / 1000;
    let ms = dur % 1000;
    color("duration") + secs + "." + ms + "s" + color("") + " "
} else {
    color("duration") + dur + "ms" + color("") + " "
}
