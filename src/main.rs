fn main() {
    let seed = std::env::args()
        .nth(1)
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(42);
    xenolab::ui::run(seed).unwrap();
}
