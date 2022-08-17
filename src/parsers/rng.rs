use rand::rngs::SmallRng;
use rand::SeedableRng;

pub fn get() -> SmallRng {
    SmallRng::from_rng(rand::thread_rng()).unwrap_or_else(|_| SmallRng::from_entropy())
}
