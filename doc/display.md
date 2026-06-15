## Display
Display is the view layer — it knows nothing about internal logic.

```rust
pub enum SessionEvent {
    Update(WeekTotals),
    History(WeekTotals),
}

pub struct WeekTotals {
    pub minutes: u32,
    pub steps: u32,
}
```

`symbol.rs` defines the seven-segment digit layout — the contract for rendering time and step values as pixel rectangles.
