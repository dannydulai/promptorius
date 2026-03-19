// Show  and branch name if in a git repo
if !git_is_repo() {
    return ();
}

let branch = git_branch();
color("git_branch") + " " + branch + color("")
