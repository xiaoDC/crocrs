c:
  #cargo check
  cargo clippy -p peerdiscovery


f:
  cargo fmt --all


o:
  cargo outdated


s:
  RUST_LOG=DEBUG \
  cargo run -- send xxxxxxxxxx --relay 2222222


r:
  cargo run -- xxxxxxxxxx relay 2222222


t:
  cargo test -p dag -- --nocapture
  #cargo test pdf::pdf_test -- --nocapture


unuse:
  cargo +nightly udeps --all-targets
