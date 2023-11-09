# Merkle mountain range
[![Crates.io](https://img.shields.io/crates/v/ckb-merkle-mountain-range.svg)](https://crates.io/crates/ckb-merkle-mountain-range)

A generalized merkle mountain range implementation.

## Features

* Leaves accumulation
* Multi leaves merkle proof
* Accumulate from last leaf's merkle proof

## Construct

``` txt
# An 11 leaves MMR

          14
       /       \
     6          13
   /   \       /   \
  2     5     9     12     17
 / \   /  \  / \   /  \   /  \
0   1 3   4 7   8 10  11 15  16 18
```

In MMR, we use the insertion order to reference leaves and nodes.

We insert a new leaf to MMR by the following:

1. insert leaf or node to next position.
2. if the current position has a left sibling, we merge the left and right nodes to produce a new parent node, then go back to step 1 to insert the node.

For example, we insert a leaf to the example MMR:

1. insert leaf to next position: `19`.
2. now check the left sibling `18` and calculate parent node: `merge(mmr[18], mmr[19])`.
3. insert parent node to position `20`.
4. since the node `20` also has a left sibling `17`, calculate parent node: `merge(mmr[17], mmr[20])`.
5. insert new node to next position `21`.
6. since the node `21` have no left sibling, complete the insertion.

Example MMR after insertion of a new leaf:

``` txt
          14
       /       \
     6          13            21
   /   \       /   \         /   \
  2     5     9     12     17     20
 / \   /  \  / \   /  \   /  \   /  \
0   1 3   4 7   8 10  11 15  16 18  19
```

## Merkle root

An MMR is constructed by one or more sub merkle trees (or mountains). Each sub merkle tree's root is a peak in MMR, we calculate the MMR root by bagging these peaks from right to left.

For example, in the 11 leaf MMR we have 3 peaks: `14, 17, 18`, we bag these peaks from right to left to get the root: `merge(mmr[14], merge(mmr[17], mmr[18]))`.

## Merkle proof

The merkle proof is an array of hashes constructed with the following parts:

1. A merkle proof from the leaf's sibling to the peak that contains the leaf.
2. A hash that bags all right-hand side peaks, skip this part if no right-hand peaks.
3. Hashes of all left-hand peaks from right to left, skip this part if no left-hand peaks.

We can reconstruct the merkle root from the proofs. Pre-calculating the peak positions from the size of MMR may help us do the bagging.

## References

* [Merkle mountain range](https://github.com/opentimestamps/opentimestamps-server/blob/master/doc/merkle-mountain-range.md)
* [Grin Doc](https://github.com/mimblewimble/grin/blob/master/doc/mmr.md#structure)
