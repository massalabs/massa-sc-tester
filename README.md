# massa-sc-tester

`massa-sc-tester` is a minimal testing environment made for running massa smart contracts. This environment provides an execution config, a ledger and a trace output. The trace is an in-depth description of every step, ABI call and asynchronous messages execution.

## Execution config

The default `execution_config.json` contains an example of every available step.

## Running the tester

```
cargo run -- execution_config.json
```
