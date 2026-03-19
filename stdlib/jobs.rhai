// Show ҂ if there are background jobs
let count = env("PROMPTORIUS_JOBS");

if count == "" || count == "0" {
    return ();
}

" " + color("jobs") + "҂" + color("")
