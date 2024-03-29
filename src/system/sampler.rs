use std::collections::HashSet;

use peroxide::fuga::*;
use snafu::{ensure, OptionExt, Snafu};

use crate::{async_trait, crypto::key::exchange::PublicKey};

#[derive(Snafu, Debug)]
#[snafu(visibility(pub))]
/// Error returned when sampling fails
pub enum SampleError {
    #[snafu(display("unable to compute size from iterator"))]
    /// Unable to compute size from supplied iterator
    BadIterator,
    #[snafu(display(
        "too few peers ({}) to achieve required sample size ({})",
        actual,
        expected
    ))]
    /// Amount of keys is too low to satisfy expected size
    TooSmall {
        /// The requested sample size
        expected: usize,
        /// The actual size of set we were sampling from
        actual: usize,
    },
}

/// Trait used when sampling a set of known peers
/// # FIXME
/// allocation for the output sample is required pending stabilisation of impl trait in type alias
/// https://github.com/rust-lang/rust/issues/63063
#[async_trait]
pub trait Sampler: Send + Sync {
    /// Take a sample of keys from the provided `Sender`
    async fn sample<I: Iterator<Item = PublicKey> + Send>(
        &self,
        keys: I,
        expected: usize,
    ) -> Result<HashSet<PublicKey>, SampleError> {
        let actual: usize = keys.size_hint().1.context(BadIterator)?;

        ensure!(expected <= actual, TooSmall { expected, actual });

        self.sample_unchecked(keys, expected, actual).await
    }

    /// Sample from a set with a given list of exclusions
    async fn sample_excluding<I>(
        &self,
        keys: I,
        excluding: &[PublicKey],
        expected: usize,
    ) -> Result<HashSet<PublicKey>, SampleError>
    where
        I: IntoIterator<Item = PublicKey> + Send,
        I::IntoIter: Send,
    {
        let sample = keys.into_iter().filter(|x| !excluding.contains(x));

        self.sample(sample, expected).await
    }

    /// Takes a sample from an `Iterator` already knowing its bounds.
    /// This is the only method that should be implemented in custom `Sampler`s
    async fn sample_unchecked<I: Iterator<Item = PublicKey> + Send>(
        &self,
        keys: I,
        expected: usize,
        total: usize,
    ) -> Result<HashSet<PublicKey>, SampleError>;
}

/// A naive sampler using Poisson sampling
#[derive(Clone, Copy)]
pub struct PoissonSampler {}

impl Default for PoissonSampler {
    fn default() -> Self {
        Self {}
    }
}

#[async_trait]
impl Sampler for PoissonSampler {
    async fn sample_unchecked<I: Iterator<Item = PublicKey> + Send>(
        &self,
        keys: I,
        expected: usize,
        size: usize,
    ) -> Result<HashSet<PublicKey>, SampleError> {
        let prob = expected as f64 / size as f64;
        let sampler = Bernoulli(prob);
        let mut sample = sampler.sample(size as usize);

        Ok(keys
            .filter(move |_| {
                if let Some(x) = sample.pop() {
                    (x - 1.0).abs() < f64::EPSILON
                } else {
                    false
                }
            })
            .collect())
    }
}

#[derive(Clone, Copy)]
/// Sampler that selects all known keys instead of sampling randomly
pub struct AllSampler {}

impl Default for AllSampler {
    fn default() -> Self {
        Self {}
    }
}

#[async_trait]
impl Sampler for AllSampler {
    async fn sample_unchecked<I: Iterator<Item = PublicKey> + Send>(
        &self,
        keys: I,
        _: usize,
        _: usize,
    ) -> Result<HashSet<PublicKey>, SampleError> {
        Ok(keys.collect())
    }
}

#[cfg(test)]
mod test {
    use super::{
        super::{
            sampler::{AllSampler, PoissonSampler},
            sender::CollectingSender,
            Sender,
        },
        *,
    };
    use crate::test::*;

    static EXPECTED: usize = 100;
    static ROUNDS: usize = 100;

    macro_rules! sampling_test {
        ($sampler:ty, $size:expr, $lower:expr, $upper:expr) => {
            let mut total = 0;

            for _ in 0..ROUNDS {
                let size = $size;
                let sender = CollectingSender::new(keyset(size));
                let sample =
                    test_sampler::<$sampler, _>(sender, size / 2).await;

                total += sample.len();
            }

            let average = total / ROUNDS;

            assert!(average >= $lower);
            assert!(average <= $upper);
        };
    }

    async fn test_sampler<D: Default + Sampler, S: Sender<()>>(
        sender: S,
        expected: usize,
    ) -> HashSet<PublicKey> {
        let keys = sender.keys().await;

        D::default()
            .sample(keys.iter().copied(), expected)
            .await
            .expect("sampling failed")
    }

    #[tokio::test]
    async fn exclusions() {
        let mut keys = keyset(10);
        let exclusions = keys.by_ref().take(5).collect::<Vec<_>>();
        let keys = keys.collect::<Vec<_>>();

        let sampler = AllSampler::default();

        let sample = sampler
            .sample_excluding(keys.iter().copied(), &exclusions, keys.len())
            .await
            .expect("sampling failed");

        assert_eq!(sample.len(), keys.len());
    }

    #[tokio::test]
    async fn poisson() {
        sampling_test!(
            PoissonSampler,
            EXPECTED,
            EXPECTED / 2 - 5,
            EXPECTED / 2 + 5
        );
    }

    #[tokio::test]
    async fn all() {
        sampling_test!(AllSampler, EXPECTED, EXPECTED, EXPECTED);
    }
}
