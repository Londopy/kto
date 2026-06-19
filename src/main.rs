//! KTO v3 binary entry point. All logic lives in the library (`kto::run`) so it
//! is reachable from the integration tests under `tests/`.

fn main() {
    kto::run();
}
