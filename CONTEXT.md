# Anoma Kind Tables

The kind tables the Anoma protocol adapters are committed to, one per chain per environment, and the supported-token list they are built from.

## Language

**Kind**:
The hash of a resource's logic ref and label ref, as an elliptic curve point. A resource's commitment covers both, so its kind is fixed when it is created. A kind table is indexed by kind, written as the pair.
_Avoid_: type, asset, denomination, key

**Kind point**:
The point the compliance circuit uses as the binding generator for a resource's quantity in the delta: the one the table in force assigns to the resource's kind, or the kind itself when the table has no entry for it. Two resources balance against each other if and only if their kind points are equal.
_Avoid_: point (on its own), generator

**Logic ref**:
The verifying key of the circuit that governs a resource. With the label ref, it fixes the resource's kind.
_Avoid_: circuit ID, image ID, VK (all correct upstream, but this repo says logic ref because that is the field name a kind is written with)

**Label ref**:
The other half of what a kind is hashed from, distinguishing instances that share one logic — for an ERC20 resource, the forwarder and token it belongs to.

**Kind table**:
The mapping from kinds to kind points that one protocol adapter is committed to. Exactly one per protocol adapter, and therefore one per chain per environment.
_Avoid_: kind registry, kind map, lookup table

**Chain table**:
A kind table named by the chain it belongs to. The unit this repo generates, reviews and publishes.

**Entry**:
One row of a kind table: a kind, written as its `(logic ref, label ref)`, and the kind point it is assigned.

**Fungibility domain**:
The resources whose kind points are equal, and the kinds the table assigns that kind point to. On this table that is one forwarder holding one token on one chain, together with every kind whose resources those tokens back. Their kind point is the kind of the active circuit version under the current forwarder's label, so that version's entries are assigned their own kind, and every other member is an alias of them.
_Avoid_: domain, alias set, anchor

**Member**:
An entry inside a fungibility domain: one circuit version under one forwarder's label, assigned the fungibility domain's kind point. All but the active version's own entries are aliases.

**Alias**:
An entry assigned another kind as its kind point, not its own. The table makes two kinds share one kind point, so their resources are fungible. Every member of a fungibility domain is one, except the active version's own entries. Its `alias_of` names the kind it takes its kind point from. Only the table can express an alias, and an alias is a permission to create tokens of its fungibility domain: the only entry a reviewer must read.
_Avoid_: override, remap, redirect, canonical (for the others: an entry assigned its own kind)

**Circuit version**:
One release of the ERC20 transfer circuit, listed in `data/circuit-versions.json` with its logic ref and a status. Exactly one version is **active**: it keeps its own kind, and the backend creates its resources. Every other is **deprecated**: an alias of the active one, which the backend consumes and converts. The active version is the highest listed, and an older version never becomes active again. Every listed version is a member of every ERC20 fungibility domain, and nothing is ever removed.
_Avoid_: succession, upgrade, migration (the migration is what listing enables, not the record of it), inherit (nothing passes from one version to another — both kinds are assigned one kind point)

**Retired forwarder**:
An ERC20 forwarder whose tokens moved to the current one. Its label stays a member of every fungibility domain it backed, for the logic refs it accepted, so the resources created behind it can still convert and leave. Recorded in the forwarder repository's deployment record, never here.

**Kind table commitment**:
The digest a protocol adapter stores and every compliance proof reproduces, covering every entry and their order. The value this repo exists to publish.
_Avoid_: kind table hash, table root

**Supported token**:
An ERC20 contract this project maintains kinds for, on a named chain. Being supported is a standing commitment and does not depend on where anything is deployed.

**Environment**:
One of the two protocol adapter deployments a kind table can be installed on, each tracking a branch. Says which deployment, never which chain.
_Avoid_: network, deployment target

**Staging / Production**:
The two environments, matching the protocol adapter's. Staging tracks `staging`, production tracks `main`.

**Promotion gate**:
The assertion, run only on a pull request into an environment's branch, that every protocol adapter in that environment already stores the commitment this source computes.
