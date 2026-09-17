# NL → Vibe pair (passes `eval_cell`)

**Natural language:** Clamp `score` to the closed interval 0…100.

**Source:** `../12_1_cell.vibe`

```vibe
= math.max(0, math.min(100, score))
```

**Kind:** cell (Pure). **Must not** call `pulse.publish` or `graph.commit`.
