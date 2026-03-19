//! Host API: per-segment cache (cache_set, cache_get, cache_has, cache_del).
//!
//! Cache is namespaced per segment and persisted across prompt renders
//! via a simple JSON file in the XDG cache directory.

use rhai::{Dynamic, Engine};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// In-memory cache store. In a full implementation, this would be
/// loaded from / persisted to `$XDG_CACHE_HOME/promptorius/cache.json`.
type CacheStore = Rc<RefCell<HashMap<String, Dynamic>>>;

pub fn register(engine: &mut Engine) {
    // TODO: namespace by segment name when called from pipeline
    let store: CacheStore = Rc::new(RefCell::new(HashMap::new()));

    let s = store.clone();
    engine.register_fn("cache_set", move |key: &str, value: Dynamic| {
        s.borrow_mut().insert(key.to_string(), value);
    });

    let s = store.clone();
    engine.register_fn("cache_get", move |key: &str| -> Dynamic {
        s.borrow().get(key).cloned().unwrap_or(Dynamic::UNIT)
    });

    let s = store.clone();
    engine.register_fn("cache_has", move |key: &str| -> bool {
        s.borrow().contains_key(key)
    });

    let s = store.clone();
    engine.register_fn("cache_del", move |key: &str| {
        s.borrow_mut().remove(key);
    });
}
