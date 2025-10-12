check:
  cargo check --workspace

clippy:
  cargo clippy --workspace

fmt:
  cargo fmt --all

wpt *ARGS:
  #!/usr/bin/env bash
  set -euo pipefail
  # Set WPT_DIR if not already set
  export WPT_DIR="${WPT_DIR:-$HOME/wpt}"
  # Verify WPT directory exists
  if [ ! -d "$WPT_DIR" ]; then
    echo "Error: WPT directory not found at $WPT_DIR"
    echo "Please clone WPT: git clone https://github.com/web-platform-tests/wpt.git ~/wpt"
    exit 1
  fi
  # Run WPT tests with structured output
  # Default to running masonry tests if no args provided
  if [ -z "{{ARGS}}" ]; then
    cd wpt/runner && cargo run --release -- css/css-grid/masonry/
  else
    cd wpt/runner && cargo run --release -- {{ARGS}}
  fi

screenshot *ARGS:
  cargo run --release --example screenshot {{ARGS}}

open *ARGS:
  cargo run --release --package readme {{ARGS}}

bump *ARGS:
  cargo run --release --package bump {{ARGS}}

todomvc:
  cargo run --release --example todomvc

small:
  cargo build --profile small -p counter --no-default-features --features cpu_backend,system_fonts