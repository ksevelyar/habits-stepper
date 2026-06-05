## Display
Display is view layer, it knows nothing about internal logic.

```rust
pub enum DisplayEvent {
    SessionUpdate { today_minutes: u16, week_minutes: u16 },
    SessionHistory { week1_minutes: u16, week2_minutes: u16, week3_minutes: u16 },
}
```
