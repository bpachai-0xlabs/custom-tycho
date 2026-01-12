1. Can AAVE be integrated into Tycho ?

- Is Aave protocol tradable liquidity or position-based ?

  - It is lending positions -> Not a natural fit for Tycho

- For Aave an extractor can be written but what is the component in case of it ?

  - Is it

    - a reserve?
    - a debt token?
    - a position?
    - a borrow quote?

- For Aave do we need simulation or just state?

  - If we only want:

        - rates
        - liquidity
        - borrow caps

    _Then there is no need of Tycho simulation, Tycho indexer is sufficient and it can be used as a state indexer_ (HOW?)

  - If we need

    - atomic borrow -> swap -> repay
    - leverage loops
    - solver-style strategies

  - Then, we will have to develop:

    - extractor
    - simulation adapter
    - executor

- Aave would almost certainly start as, VM protocol (bytecode execution) which would massively change _performance, complexity, expectations_

_To integrate a custom protocol into Tycho, we usually need to write an extractor in the Tycho indexer, **but only after deciding how that protocol maps to Tycho’s “component + simulation + execution” model**._

Tycho indexer has 3 storage layers:

1. Ephemeral in-memory state

- Primary state that lives in memory, updated block by block, reorg safe, used to generate snapshots and deltas (which is then streamed to client). If the indexer starts it rebuilds state from start and does not "load a database and continue"

2. Write-ahead/persistence

- It can be RocksDB, LMDB, filesystem logs, checkpoints but these are implementation details, not queryable by clients, they exist for restart speed, crash recovery and operational stability.

3. Client-side storage (our responsibility)

- Once data leaves indexer, Tycho forgets about it, the client must store snapshots, apply deltas and maintain its own DB if needed.

- Tycho indexer is stateless from the client’s point of view.

```java
Tycho Indexer = "State compiler"
Tycho Client  = "Trading brain"
```

- Indexer:

  - reads chain
  - reconstructs protocol state
  - emits snapshots + deltas

- Client:

  - stores state
  - simulates trades
  - chooses routes
  - encodes execution
  - submits transactions

_So in terms of Aave we must first define what that component is and based on that the extractor will be written and as per the extractor logic the indexer will read chain, reconstructs protocol state and emit snapshots + deltas which would then be streamed to clients and then client will decide whether to store state, simulate a borrow or repay ? or encode execution or sumbit transaction ? If that's the case then what will be the component for Aave but before that does Aave even fit into Tycho ?_

- In case of Aave, "simulating a borrow or repay" is misleading in terms of Tycho, how?

  - The above assumes:

    - borrow/repay is a pure function of current state
    - execution is atomic
    - post-trade state is locally predictable

_The above assumption is false for Aave_ this brings us to the real question...

- Does Aave even fit into Tycho?

  - Aave does not naturally fit into Tycho's model, it's a mismatch of abstractions, why Tycho exists?: Tycho is designed for protocols where:

    1.

    ```text
    amount_out = f(amount_in, state)
    ```

    2. Execution is atomic
    3. State transition is local and deterministic
    4. **There is a clear component with tradable liquidity** (Pools in case of AMMs)

  - Aave violates all the above assumptions...

    - No stateless quote function
    - For swaps:

    ```text
    get_amount_out(amount_in, pool_state)
    ```

    - For Aave borrow:

    ```text
    borrowable_amount =
    f(
      user_collateral,
      user_debt,
      LTV,
      liquidation_threshold,
      price_oracles,
      eMode,
      interest_indices,
      isolation_mode,
      caps,
      reserve_frozen,
      health_factor
    )
    ```

    This is not a component-local function, it is _user specific, oracle dependent, cross reserve, time dependent_ **Tycho simulation assumes no user context** This alone breaks the model.

    - Execution is not atomic in the Tycho sense

      - A swap:

        - tokens in -> tokens out
        - pool state updates
        - done

    - Aave borrow:

      - touches:

        - user account
        - debt token
        - reserve
        - interest index
        - oracle snapshot

    - may revert due to external conditions
    - interacts with global protocol state

    - There is no obvious "component"

      - For Uniswap: component = pool
      - For Aave: Reserve, it is not an independent component, user position leads to infinite components, extractor becomes user-indexer, tycho becomes a wallet tracker (not a good idea), market configuration is also broken as you cannot simulate without user state.

- So... does Aave fit into Tycho?

  - Aave does not fit Tycho's core abstraction

  - Tycho is:

    - pool-centric
    - stateless
    - quote-first
    - solver-friendly

  - Aave is:

    - account-centric
    - oracle-drive
    - stateful over time
    - risk-engine based

- When could Aave make sense in Tycho?

  - First, Tycho as a state feeder, not a simulator
  - Tycho indexer could: stream -> reserve liquidity, interest rates, caps, oracle prices, clients consume this data and simulation happens elsewhere.

  _In case of uniswap pool since the mathematics is straightforward and independent of other contract states, hence tycho simulation works whereas in case of Aave that is not the case hence tycho simulation won't work, right?_

  ```text
  Tycho simulation works for Uniswap because swaps are component-local, stateless, atomic, and composable, whereas Aave borrow/repay operations are user-dependent, cross-component, oracle-driven, and time-dependent, which violates Tycho’s simulation model.
  ```
