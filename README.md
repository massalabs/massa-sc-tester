# massa-sc-tester

`massa-sc-tester` is a minimal testing environment made for running massa smart contracts. This environment provides a ledger and a call stack that can be interacted with through a smart contract, as you would in the Massa network.

## What it does *not* provide

`massa-sc-tester` is not a network. Some ABI calls that require blockchain data other than the ledger or the call stack have dummy implementations such as:

* `send_message()`
* `get_current_period()`
* `get_current_thread()`

## Running a smart contract in the tester

Basic usage:
```
cargo run path/to/smart-contract.wasm
```

Additional command-line options:
```
function={function_name} // default value 'main'

param={function_param}

addr={caller_address}

coins={caller_raw_coins} // default value '0', 1 raw_coin = 1e-9 coin
```

Example:
```
cargo run test.wasm function=display param=hello
```

Note that the ledger state is written in `ledger.json` after each execution. `massa-sc-tester` reads it before new executions.
