pub mod fixed_update;
pub mod late_update;
pub mod prerender;
pub mod start;
pub mod update;

pub trait HasPriority {
    fn priority(&self) -> u32;
}
