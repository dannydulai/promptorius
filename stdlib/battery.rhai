// Show battery status when discharging or below threshold
let pct = battery_pct();

if pct < 0 { return (); }  // no battery

let state = battery_state();
let threshold = config.get("threshold") ?? 100;

if state == "full" || (state == "charging" && pct >= threshold) {
    return ();
}

let icon = if pct > 75 { "█" }
    else if pct > 50 { "▆" }
    else if pct > 25 { "▄" }
    else if pct > 10 { "▂" }
    else { "▁" };

let col = if pct <= 10 { "battery-critical" }
    else if pct <= 25 { "battery-low" }
    else { "battery-normal" };

let suffix = if state == "charging" { "⚡" } else { "" };

color(col) + icon + " " + pct + "%" + suffix + color("")
