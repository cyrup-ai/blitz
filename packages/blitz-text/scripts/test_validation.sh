#!/bin/bash
# Test validation script for blitz-text package
# Validates that extracted tests work correctly with nextest

set -e

echo "🧪 Validating extracted tests..."

# Check if nextest is installed
if ! command -v cargo-nextest &> /dev/null; then
    echo "Installing cargo-nextest..."
    cargo install cargo-nextest --locked
fi

echo "📋 Running test compilation check..."
cargo check --tests

echo "🔧 Running standard cargo tests..."
cargo test --no-run

echo "⚡ Running nextest (extracted tests)..."
cargo nextest run --profile default

echo "🎯 Running specific extracted test files..."
cargo nextest run --profile default --package blitz-text \
    cosmyc_types_tests \
    gpu_cache_tests \
    text_measurer_tests \
    text_system_tests \
    shape_cache_tests \
    enhanced_measurement_tests \
    gpu_viewport_tests \
    cosmyc_editor_tests \
    measurement_core_tests \
    gpu_text_atlas_tests

echo "📊 Running performance profile tests..."
cargo nextest run --profile perf --package blitz-text \
    text_measurer_tests::test_caching

echo "✅ All extracted tests validated successfully!"

echo "📈 Test extraction summary:"
echo "  ✅ cosmyc_types.rs tests → tests/cosmyc_types_tests.rs"
echo "  ✅ gpu/cache.rs tests → tests/gpu_cache_tests.rs" 
echo "  ✅ measurement/mod.rs tests → tests/text_measurer_tests.rs"
echo "  ✅ text_system.rs tests → tests/text_system_tests.rs"
echo "  ✅ cosmyc/shape_cache.rs → tests/shape_cache_tests.rs"
echo "  ✅ measurement/enhanced.rs → tests/enhanced_measurement_tests.rs"
echo "  ✅ gpu/viewport.rs → tests/gpu_viewport_tests.rs"
echo "  ✅ cosmyc/editor.rs → tests/cosmyc_editor_tests.rs"
echo "  ✅ measurement/core.rs → tests/measurement_core_tests.rs"
echo "  ✅ gpu/text_atlas.rs → tests/gpu_text_atlas_tests.rs"
echo ""
echo "🏆 ALL 10 MODULES EXTRACTED - 100% COMPLETE!"
echo "📂 Module rename: cosmyc_text_integration → cosmyc"
echo ""
echo "🚀 Nextest profiles configured:"
echo "  - default: 8 threads, 1 retry, comprehensive output"
echo "  - ci: 4 threads, 3 retries, fail-fast for CI/CD"
echo "  - perf: 1 thread, 0 retries, for performance testing"
echo ""
echo "💡 To run tests with nextest:"
echo "  cargo nextest run --profile default    # Standard run"
echo "  cargo nextest run --profile ci         # CI/CD run" 
echo "  cargo nextest run --profile perf       # Performance run"