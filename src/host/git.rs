//! Host API: native git operations via libgit2 (git2 crate).

use rhai::{Dynamic, Engine, Map};

pub fn register(engine: &mut Engine) {
    engine.register_fn("git_is_repo", || -> bool {
        git2::Repository::discover(".").is_ok()
    });

    engine.register_fn("git_branch", || -> String {
        let repo = match git2::Repository::discover(".") {
            Ok(r) => r,
            Err(_) => return String::new(),
        };

        if let Ok(head) = repo.head() {
            if let Some(name) = head.shorthand() {
                return name.to_string();
            }
        }

        // Detached HEAD — return short SHA
        if let Ok(head) = repo.head() {
            if let Some(oid) = head.target() {
                let hex = oid.to_string();
                return hex[..7.min(hex.len())].to_string();
            }
        }

        String::new()
    });

    engine.register_fn("git_root", || -> String {
        git2::Repository::discover(".")
            .ok()
            .and_then(|r| r.workdir().map(|p| p.to_string_lossy().into_owned()))
            .unwrap_or_default()
    });

    engine.register_fn("git_status", || -> Map {
        let mut map = Map::new();
        map.insert("modified".into(), Dynamic::from(0_i64));
        map.insert("staged".into(), Dynamic::from(0_i64));
        map.insert("untracked".into(), Dynamic::from(0_i64));
        map.insert("conflicts".into(), Dynamic::from(0_i64));
        map.insert("ahead".into(), Dynamic::from(0_i64));
        map.insert("behind".into(), Dynamic::from(0_i64));

        let repo = match git2::Repository::discover(".") {
            Ok(r) => r,
            Err(_) => return map,
        };

        let statuses = match repo.statuses(None) {
            Ok(s) => s,
            Err(_) => return map,
        };

        let mut modified = 0_i64;
        let mut staged = 0_i64;
        let mut untracked = 0_i64;
        let mut conflicts = 0_i64;

        for entry in statuses.iter() {
            let s = entry.status();
            if s.is_conflicted() {
                conflicts += 1;
            } else if s.is_wt_new() {
                untracked += 1;
            } else {
                if s.intersects(
                    git2::Status::INDEX_NEW
                        | git2::Status::INDEX_MODIFIED
                        | git2::Status::INDEX_DELETED
                        | git2::Status::INDEX_RENAMED
                        | git2::Status::INDEX_TYPECHANGE,
                ) {
                    staged += 1;
                }
                if s.intersects(
                    git2::Status::WT_MODIFIED
                        | git2::Status::WT_DELETED
                        | git2::Status::WT_TYPECHANGE
                        | git2::Status::WT_RENAMED,
                ) {
                    modified += 1;
                }
            }
        }

        map.insert("modified".into(), Dynamic::from(modified));
        map.insert("staged".into(), Dynamic::from(staged));
        map.insert("untracked".into(), Dynamic::from(untracked));
        map.insert("conflicts".into(), Dynamic::from(conflicts));

        // ahead/behind requires comparing to upstream
        if let Ok(head) = repo.head() {
            if let Some(local_oid) = head.target() {
                if let Ok(branch) = repo.find_branch(
                    head.shorthand().unwrap_or(""),
                    git2::BranchType::Local,
                ) {
                    if let Ok(upstream) = branch.upstream() {
                        if let Some(upstream_oid) = upstream.get().target() {
                            if let Ok((ahead, behind)) =
                                repo.graph_ahead_behind(local_oid, upstream_oid)
                            {
                                map.insert("ahead".into(), Dynamic::from(ahead as i64));
                                map.insert("behind".into(), Dynamic::from(behind as i64));
                            }
                        }
                    }
                }
            }
        }

        map
    });
}
