1. A Tycho client subscribes to **Tycho indexer streams**, which exposes normalized protocol state for supported protocol systems. _(uniswap_v2, uniswap_v3, ekubo_v2, pancakeswap_v3, sushiswap_v2, uniswap_v4_hooks, vm:curve, vm:balancer_v2, vm:maverick_v2, pancakeswap_v2, fluid_v1, balancer_v3, uniswap_v4, rocketpool)_

   1.1 The Tycho indexer handles **chain access, decoding, reorg handling & protocol-specific logic**

   1.2 The client handles filtering, state assembly, routing, quoting and execution logic

2. The snapshots are produced by the _tycho indexer_, which are then consumed by _tycho client_. What actually happens is..

```pgsql
Ethereum node
  └─ traces / logs / storage
      └─ Tycho indexer
          └─ Extractors (These are responsible for getting all data from chain)
              └─ Snapshot + Delta stream
                  └─ Your client
```

3. The **tycho client** also receives deltas, removals per block and then applies those deltas, removals to in memory db or persistent db.

4. **Snapshots** contain the complete state required by Tycho to simulate and execute trades for the selected components (components in regards to AMM are the pool contracts)

5. The **extractors** has the logic for getting data from the protocol system in the tycho indexer.
