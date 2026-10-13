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

## tests

`cargo test`. there's one. it passes.
