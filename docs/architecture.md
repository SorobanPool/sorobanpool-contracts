# Architecture
Six contracts (config, registry, group_buy, disputes, supplier_bond, reputation) share types and math in `sp_common`. See the build brief sections 3, 4 and 6 for the normative rules. Contracts are the source of truth for escrow; the backend cannot move funds.
