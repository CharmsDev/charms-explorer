Execute tasks in roder and modify the status when done
Read the context file to get additional info at: /Users/ricartjuncadella/Documents/Prj/bitcoinos/charms-explorer/indexer/_rjj/context.md


# Task 1 | status: completed
the charms-explorer/api/src/services/diagnostic.rs shows that all works in the api, so it should be working as well in the indexer charms-explorer/indexer/src:
{"bitcoin_rpc_test":{"best_block_hash":"00000000cc8885c7c130f693cd0efabd5890cde93f37f71648fece8d58733c7d","block_count":81370,"host":"bitcoind-t4-test.fly.dev","port":"48332","status":"connected"},"connection":{"backend":"PostgreSQL","status":"connected","version":"PostgreSQL 17.2 (Ubuntu 17.2-1.pgdg24.04+1) on x86_64-pc-linux-gnu, ...
I need to know that the indexer is working all the time. So can we save a timestamp in the db, so we now what was the last execution of the bucle, and the block number ?

# Task 2 | status: completed
I need a dahsboard in the indexer where I can view what ios the current status of the indexer. I want to last blocks indexed, which ones are confirmed, how many charms do we have in total. And something else if you think its interesting accroding to the data we have now. If we need to store more data, ask me first.
In this task, create the api endpoint to get the summary

# Task 3 | status: completed
in this task, create the design needed to see the summary data in the indexer.
Add a button on top of the header to access the status page.

# Final Task | status: completed
Write relevant information to the context file.
Write commands that may be suitable to be reexecuted in future taks, like docker commands, sql commands, api requests, other curk requests, etc, taken from this current task cline history.
Avoid repeating commands in make file or Readme. just mention that we have them.
