// Show current time
// Uses the format from segment config, defaulting to %H:%M
let fmt = config.get("format") ?? "%H:%M";
exec("date", ["+" + fmt])
