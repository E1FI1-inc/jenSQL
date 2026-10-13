# jenSQL

itz a DATABEES. sortus of. wan FIEL of RUSTUS, no DEPZEL.

## run it

```
cargo run              # uses jensql.db
cargo run -- other.db  # if you must
```

type `EXIT` when you're bored.

## the words

normal SQL was taken so we fixed the spelling.

```
KREARTE TABLE cats (name, lives)
INSZS INTO cats VALUES ('Tom', 9), ('Garfield', 1)
SELEL * FROMM cats WHERE lives > 2 AND name != 'Tom'
UPDEED cats SET lives = 8 WHERE name = 'Tom'
DELEL FROMM cats WHERE lives < 5
ALTUS TABLE cats ADD owner      -- or DROP owner
DROLOLP TABLE cats
EIFLEL                          -- nukes every table. no "are you sure". you were sure.
```

`SELECT` won't work. that's on purpose. mostly.

## things it doesn't do

OR, ORDER BY, JOIN, types, indexes, transactions, scaling, your taxes.
one command per line. the whole file gets rewritten after every command, so maybe don't put your startup in here.

## why it's slow

also on purpose. every decision picks the worst honest option, always, no switch:

- no indexes, no dirty flag, no plan cache, no result cache. ever.
- the whole db gets rewritten after every command, and again after every error.
- every byte is written through its own open + append + fsync, plus three backup copies.
- the whole db file is re-read from disk once per row comparison.
- the query gets re-tokenized for every row, and every value is re-parsed every time.
- inserts rebuild the entire table per row; selects deep-copy the entire db first.
- the parser clones its whole token vector on every single token read.

real syscalls and real algorithms only. no sleeping, no faking, it just does the maximum work.

## tests

`cargo test`. there's one. it passes. it is patient.
