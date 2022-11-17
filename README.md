# massa-sc-tester

`massa-sc-tester` is a minimal testing environment made for running massa smart contracts. This environment provides an execution config, a ledger and a trace output. The trace is an in-depth description of every step, ABI call and asynchronous messages execution.

## Execution config

The default `execution_config` files (available in `json` and `yaml` format) contain an example of every available step.

## Running massa-sc-tester

As the default `execution_config` files use a smart contract from [`massa-sc-examples`](https://github.com/massalabs/massa-sc-examples), you need to build it first: 

Setup:

```
git clone https://github.com/massalabs/massa-sc-tester.git
git clone https://github.com/massalabs/massa-sc-examples.git
cd massa-sc-examples/async-calls
npm install
npm run build
cd ../../massa-sc-tester
```

Run:

```
cargo run -- execution_config.json
```
```
cargo run -- execution_config.yaml
```
