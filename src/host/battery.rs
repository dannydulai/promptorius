//! Host API: battery functions (battery_pct, battery_state, battery_time).

use rhai::Engine;

pub fn register(engine: &mut Engine) {
    engine.register_fn("battery_pct", || -> i64 {
        get_battery()
            .map(|b| (b.state_of_charge().value * 100.0) as i64)
            .unwrap_or(-1)
    });

    engine.register_fn("battery_state", || -> String {
        get_battery()
            .map(|b| match b.state() {
                starship_battery::State::Charging => "charging".to_string(),
                starship_battery::State::Discharging => "discharging".to_string(),
                starship_battery::State::Full => "full".to_string(),
                starship_battery::State::Empty => "empty".to_string(),
                _ => "unknown".to_string(),
            })
            .unwrap_or_else(|| "none".to_string())
    });

    engine.register_fn("battery_time", || -> i64 {
        get_battery()
            .and_then(|b| match b.state() {
                starship_battery::State::Discharging => b.time_to_empty(),
                starship_battery::State::Charging => b.time_to_full(),
                _ => None,
            })
            .map(|t| t.value as i64)
            .unwrap_or(-1)
    });
}

fn get_battery() -> Option<starship_battery::Battery> {
    let manager = starship_battery::Manager::new().ok()?;
    manager.batteries().ok()?.next()?.ok()
}
