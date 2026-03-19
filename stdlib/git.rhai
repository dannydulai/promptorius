if !git_is_repo() {
    return ();
}

let branch = git_branch();
let max_len = config.get("max_branch_len") ?? 40;

if branch.len() > max_len {
    branch = branch[..max_len] + "…";
}

let s = git_status();
let indicators = "";

if s.staged > 0    { indicators += color("git-staged") + "+"; }
if s.modified > 0  { indicators += color("git-modified") + "!"; }
if s.untracked > 0 { indicators += color("git-untracked") + "?"; }
if s.conflicts > 0 { indicators += color("git-conflicts") + "✘"; }
if indicators != "" { indicators += color(""); }

let remote = "";
if s.ahead > 0  { remote += "↑" + s.ahead; }
if s.behind > 0 { remote += "↓" + s.behind; }

color("git-branch") + " " + branch + " " + indicators + remote
