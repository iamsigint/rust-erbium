#!/bin/bash

# Erbium Blockchain Performance Benchmark Script
# Tests the performance optimizations implemented

set -e

echo "🚀 Erbium Blockchain Performance Benchmark"
echo "=========================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="$PROJECT_ROOT/target/release"
BENCHMARK_DIR="$PROJECT_ROOT/benchmarks"
LOG_FILE="$BENCHMARK_DIR/benchmark_$(date +%Y%m%d_%H%M%S).log"

# Create benchmark directory
mkdir -p "$BENCHMARK_DIR"

# Logging function
log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') - $*" | tee -a "$LOG_FILE"
}

# Check if binary exists
check_binary() {
    if [ ! -f "$TARGET_DIR/erbium-blockchain" ]; then
        log "${RED}Error: Binary not found at $TARGET_DIR/erbium-blockchain${NC}"
        log "Please build the project first with: cargo build --release"
        exit 1
    fi
}

# Build project if needed
build_project() {
    log "${BLUE}Building project in release mode...${NC}"
    cd "$PROJECT_ROOT"
    cargo build --release
    log "${GREEN}Build completed${NC}"
}

# Run cache performance test
test_cache_performance() {
    log "${BLUE}Testing cache performance...${NC}"

    # Test cache hit ratio
    log "Running cache benchmark..."
    timeout 30s "$TARGET_DIR/erbium-blockchain" --bench-cache 2>>"$LOG_FILE" || true

    # Test memory usage
    log "Testing memory usage..."
    /usr/bin/time -v "$TARGET_DIR/erbium-blockchain" --test-memory 2>>"$LOG_FILE" || true

    log "${GREEN}Cache performance test completed${NC}"
}

# Run database performance test
test_database_performance() {
    log "${BLUE}Testing database performance...${NC}"

    # Test write performance
    log "Testing database write performance..."
    timeout 60s "$TARGET_DIR/erbium-blockchain" --bench-db-write 2>>"$LOG_FILE" || true

    # Test read performance
    log "Testing database read performance..."
    timeout 60s "$TARGET_DIR/erbium-blockchain" --bench-db-read 2>>"$LOG_FILE" || true

    # Test batch operations
    log "Testing batch operations..."
    timeout 60s "$TARGET_DIR/erbium-blockchain" --bench-db-batch 2>>"$LOG_FILE" || true

    log "${GREEN}Database performance test completed${NC}"
}

# Run state management performance test
test_state_performance() {
    log "${BLUE}Testing state management performance...${NC}"

    # Test account operations
    log "Testing account operations..."
    timeout 45s "$TARGET_DIR/erbium-blockchain" --bench-state 2>>"$LOG_FILE" || true

    # Test transaction processing
    log "Testing transaction processing..."
    timeout 45s "$TARGET_DIR/erbium-blockchain" --bench-tx 2>>"$LOG_FILE" || true

    log "${GREEN}State management performance test completed${NC}"
}

# Run consensus performance test
test_consensus_performance() {
    log "${BLUE}Testing consensus performance...${NC}"

    # Test validator selection
    log "Testing validator selection..."
    timeout 30s "$TARGET_DIR/erbium-blockchain" --bench-consensus 2>>"$LOG_FILE" || true

    # Test block validation
    log "Testing block validation..."
    timeout 30s "$TARGET_DIR/erbium-blockchain" --bench-validation 2>>"$LOG_FILE" || true

    log "${GREEN}Consensus performance test completed${NC}"
}

# Run VM performance test
test_vm_performance() {
    log "${BLUE}Testing VM performance...${NC}"

    # Test contract execution
    log "Testing contract execution..."
    timeout 60s "$TARGET_DIR/erbium-blockchain" --bench-vm 2>>"$LOG_FILE" || true

    # Test gas metering
    log "Testing gas metering..."
    timeout 30s "$TARGET_DIR/erbium-blockchain" --bench-gas 2>>"$LOG_FILE" || true

    log "${GREEN}VM performance test completed${NC}"
}

# Run network performance test
test_network_performance() {
    log "${BLUE}Testing network performance...${NC}"

    # Test P2P messaging
    log "Testing P2P messaging..."
    timeout 30s "$TARGET_DIR/erbium-blockchain" --bench-network 2>>"$LOG_FILE" || true

    # Test message compression
    log "Testing message compression..."
    timeout 30s "$TARGET_DIR/erbium-blockchain" --bench-compression 2>>"$LOG_FILE" || true

    log "${GREEN}Network performance test completed${NC}"
}

# Generate performance report
generate_report() {
    log "${BLUE}Generating performance report...${NC}"

    REPORT_FILE="$BENCHMARK_DIR/performance_report_$(date +%Y%m%d_%H%M%S).md"

    cat > "$REPORT_FILE" << 'EOF'
# Erbium Blockchain Performance Report

Generated on: $(date)

## System Information
- CPU: $(nproc) cores
- Memory: $(free -h | grep '^Mem:' | awk '{print $2}')
- Disk: $(df -h . | tail -1 | awk '{print $4}') available
- OS: $(uname -a)

## Performance Metrics

### Cache Performance
EOF

    # Extract cache metrics from log
    if grep -q "cache_hit_ratio" "$LOG_FILE"; then
        grep "cache_hit_ratio" "$LOG_FILE" | tail -5 >> "$REPORT_FILE"
    fi

    cat >> "$REPORT_FILE" << 'EOF'

### Database Performance
EOF

    # Extract database metrics
    if grep -q "db_write" "$LOG_FILE"; then
        grep "db_write\|db_read\|db_batch" "$LOG_FILE" | tail -10 >> "$REPORT_FILE"
    fi

    cat >> "$REPORT_FILE" << 'EOF'

### State Management Performance
EOF

    # Extract state metrics
    if grep -q "state_ops" "$LOG_FILE"; then
        grep "state_ops\|tx_processing" "$LOG_FILE" | tail -10 >> "$REPORT_FILE"
    fi

    cat >> "$REPORT_FILE" << 'EOF'

### VM Performance
EOF

    # Extract VM metrics
    if grep -q "vm_execution\|gas_cost" "$LOG_FILE"; then
        grep "vm_execution\|gas_cost" "$LOG_FILE" | tail -10 >> "$REPORT_FILE"
    fi

    cat >> "$REPORT_FILE" << 'EOF'

## Recommendations

Based on the benchmark results, here are the recommendations:

1. **Cache Optimization**: Ensure cache hit ratio > 80%
2. **Database Tuning**: Monitor write amplification and read latency
3. **Memory Management**: Keep memory usage under 80% of available RAM
4. **CPU Utilization**: Monitor core utilization during peak loads

## Raw Benchmark Data

See attached log file for complete benchmark output.

EOF

    log "${GREEN}Performance report generated: $REPORT_FILE${NC}"
}

# Main execution
main() {
    log "${YELLOW}Starting Erbium Blockchain Performance Benchmark${NC}"

    # Pre-flight checks
    check_binary

    # Run benchmarks
    test_cache_performance
    test_database_performance
    test_state_performance
    test_consensus_performance
    test_vm_performance
    test_network_performance

    # Generate report
    generate_report

    log "${GREEN}Performance benchmark completed successfully!${NC}"
    log "Results saved to: $LOG_FILE"
    log "Report saved to: $REPORT_FILE"
}

# Run main function
main "$@"
