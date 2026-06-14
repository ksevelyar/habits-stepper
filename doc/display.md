## Display
Display is view layer, it knows nothing about internal logic.

```rust
pub enum SessionEvent {
    Update(SessionUpdate),
    History(SessionHistory),
}

pub struct SessionUpdate {
    pub week_minutes: u32,
    pub week_steps: u32,
}

pub struct SessionHistory {
    pub prev_week_minutes: u32,
    pub prev_week_steps: u32,
}
```
