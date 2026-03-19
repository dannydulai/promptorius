let result = "";

let threshold = config.get("threshold_ms") ?? 2000;

let dur = duration();

if dur >= threshold {

    if dur >= 60000 {
        let mins = dur / 60000;
        let secs = (dur % 60000) / 1000;
        result += color("duration") + mins + "m" + secs + "s" + color("") + " "
    } else if dur >= 1000 {
        let secs = dur / 1000;
        let ms = dur % 1000;
        result += color("duration") + secs + "." + ms + "s" + color("") + " "
    } else {
        result += color("duration") + dur + "ms" + color("") + " "
    }
}

if git_is_repo() {

    let branch = git_branch();
    let max_len = config.get("max_branch_len") ?? 40;

    if branch.len() > max_len {
        branch = branch[..max_len] + "…";
    }

    let s = git_status();
    let indicators = "";

    if s.staged > 0    { indicators += color("git-staged") + " +"; }
    if s.modified > 0  { indicators += color("git-modified") + " ✎"; }
    if s.untracked > 0 { indicators += color("git-untracked") + " ?"; }
    if s.conflicts > 0 { indicators += color("git-conflicts") + " ✘"; }
    if indicators != "" { indicators += color(""); }

    let remote = "";
    if s.ahead > 0  { remote += color("git-ahead") + " ↑" + s.ahead; }
    if s.behind > 0 { remote += color("git-behind") + " ↓" + s.behind; }

    result += color("git-branch") + " " + branch + indicators + remote
}

result;
