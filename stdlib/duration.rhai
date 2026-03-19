// Show last command duration if above threshold
let threshold = config.get("threshold_ms") ?? 2000;
let dur = duration();

eprint(threshold.to_string() + "ms");
eprint(dur.to_string() + "ms");

if dur < threshold {
    return ();
}

if dur >= 60000 {
    let mins = dur / 60000;
    let secs = (dur % 60000) / 1000;
    color("warning") + mins + "m" + secs + "s" + color("")
} else if dur >= 1000 {
    let secs = dur / 1000;
    let ms = dur % 1000;
    color("warning") + secs + "." + ms + "s" + color("")
} else {
    color("muted") + dur + "ms" + color("")
}
