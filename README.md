# litechain

this is a super tiny ultra barebones blockchain that i built to learn more about blockchain eng

right now this doesn't really broadcast over a network it's based on a single machine and broadcasts between threads

you can watch it work by running test_run_blockchain in tests.rs 

expected behavior for that test:

two validators & two users are initialized

each user sends two transactions, one of each type (Stake & Transfer) to the mempool

the validators will produce two blocks & shut down once there are no more transactions in the mempool

obviously this leaves a lot to be desired, so in the future i might make validators more robust & implement actual p2p gossip across a network for building, proposing, & validating blocks 