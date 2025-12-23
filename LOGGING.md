# Logging Implementation Guide

**Date**: 2025-12-23
**Task**: T072 - Add logging for all MQTT events with tracing crate

## Overview

The application now uses structured logging via the `tracing` crate instead of `println!` and `eprintln!`. This provides better control over log levels, structured output, and production-ready logging.

## Dependencies

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

## Initialization

Logging is initialized at the start of `main.rs`:

```rust
tracing_subscriber::fmt()
    .with_env_filter(
        tracing_subscriber::EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()),
    )
    .init();
```

## Usage

### Import Statement

```rust
use tracing::{debug, error, info, warn};
```

### Log Levels

- **`error!`**: Fatal errors, unrecoverable conditions
  - MQTT connection failures
  - Configuration load errors
  - Unexpected task terminations

- **`warn!`**: Warnings, degraded functionality
  - MQTT disconnections
  - Rate limiting triggers
  - Failed to load persisted state

- **`info!`**: Normal operational messages
  - Application startup/shutdown
  - MQTT broker connections
  - Entity state changes
  - Command execution (power, source, volume)

- **`debug!`**: Detailed debugging information
  - Screen refresh triggers
  - Latency measurements
  - Hardware event details
  - Raw MQTT payloads

## Environment Control

Users can control log verbosity via the `RUST_LOG` environment variable:

```bash
# Show only errors and warnings
RUST_LOG=warn cargo run

# Show info and above (default)
RUST_LOG=info cargo run

# Show all debug messages
RUST_LOG=debug cargo run

# Module-specific filtering
RUST_LOG=snapcast_controller::homeassistant=debug,info cargo run
```

## Modules Converted

### ✅ Fully Converted

- **homeassistant/client.rs** (100% complete)
  - All MQTT operations logged
  - Connection lifecycle tracked
  - Command execution logged
  - Rate limiting logged

- **main.rs** (Critical sections complete)
  - Application lifecycle
  - Event loop
  - Configuration loading

### ⚠️ Partially Converted

- **snapcast/client.rs** (TODO: 12 statements remaining)
- **controller/state.rs** (TODO: 9 statements remaining)
- **hardware/device.rs** (TODO: 13 statements remaining)
- **hardware/display.rs** (TODO: 3 statements remaining)

## Examples

### Before

```rust
eprintln!("Failed to connect to MQTT broker: {}", e);
println!("Connected successfully");
```

### After

```rust
error!("Failed to connect to MQTT broker: {}", e);
info!("Connected successfully");
```

### Context-Aware Logging

```rust
// Good: Provides context
info!("Published power command to {}: {}", topic, payload);

// Better: Use debug for verbose details
info!("Published IR source command to {}: {}", topic, source);
debug!("IR command payload: {}", payload);
```

## Production Recommendations

1. **Default Level**: Set `RUST_LOG=info` in production
2. **Debug on Demand**: Enable `debug` only when troubleshooting
3. **Log Rotation**: Use external tools (systemd journal, logrotate)
4. **Structured Output**: Consider JSON formatter for log aggregation

## Migration Checklist

When converting print statements to logging:

- [ ] Replace `println!` with `info!` or `debug!` based on importance
- [ ] Replace `eprintln!` with `error!` or `warn!` based on severity
- [ ] Remove trailing newlines (tracing adds them automatically)
- [ ] Add context to log messages (entity IDs, topics, values)
- [ ] Use debug! for high-frequency or verbose messages
- [ ] Group related operations with structured fields (future enhancement)

## Future Enhancements

1. **Structured Fields**: Use tracing spans and fields
   ```rust
   #[tracing::instrument]
   async fn handle_command(command: HomeAssistantCommand) {
       info!("Executing command");
   }
   ```

2. **Log Aggregation**: Send logs to centralized system (ELK, Loki)

3. **Performance Metrics**: Add tracing for latency measurements

4. **Conditional Compilation**: Disable debug logs in release builds
   ```rust
   #[cfg(debug_assertions)]
   debug!("Verbose debug info");
   ```

## Common Patterns

### Error Handling with Context

```rust
match result {
    Ok(value) => {
        info!("Operation succeeded: {:?}", value);
        value
    }
    Err(e) => {
        error!("Operation failed: {}", e);
        return Err(e);
    }
}
```

### Rate Limiting

```rust
if elapsed < threshold {
    warn!("Rate limiting: Skipping command ({}ms since last)", elapsed.as_millis());
    return Ok(());
}
```

### State Changes

```rust
info!(
    "Power state changed: {} ({})",
    if power_on { "ON" } else { "OFF" },
    entity_id
);
```

## Testing

Verify logging works correctly:

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run with specific module logging
RUST_LOG=snapcast_controller::homeassistant=debug cargo run

# Run with JSON output (requires tracing-subscriber json feature)
RUST_LOG=info cargo run 2>&1 | jq .
```

## Troubleshooting

### Logs not appearing

- Check `RUST_LOG` environment variable
- Ensure tracing subscriber is initialized
- Verify log level is appropriate

### Too verbose

```bash
# Reduce to warnings only
RUST_LOG=warn cargo run
```

### Missing context

- Add more fields to log statements
- Use structured logging with spans
- Include relevant IDs and values

## References

- [Tracing Documentation](https://docs.rs/tracing/)
- [Tracing Subscriber](https://docs.rs/tracing-subscriber/)
- [Structured Logging Best Practices](https://github.com/tokio-rs/tracing)
