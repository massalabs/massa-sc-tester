# massa-sc-tester

`massa-sc-tester` is a minimal testing environment made to run massa smart contracts. This program provides a human-readable execution trace and ledger.

## Execution config

The default configuration located at `config/execution_config.yaml` contains a detailed example of what you can do with `massa-sc-tester`. The `json` format is also supported if you wish to integrate `massa-sc-tester` in another application but for human interaction the `yaml` format is recommended.

## Running massa-sc-tester

```
cargo run config/execution_config.yaml
```
