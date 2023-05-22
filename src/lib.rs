#![no_std]

/// This trait is for types with a specified means of consumption.
/// It is a counterpart to [`Drop`]. While [`Drop::drop`] takes `self`
/// by mutable reference, [`Consume::consume`] takes `self` by value.
///
/// A type must implement [`Consume`] before it can be wrapped in a
/// [`ConsumeOnDrop`].
pub trait Consume {
    /// When a [`ConsumeOnDrop<Self>`] is dropped, the underlying
    /// `Self` will be consumed using this method.
    fn consume(self);
}

pub use crate::consume_on_drop::*;
pub use crate::with_consumer::*;

mod consume_on_drop {
    use core::mem::ManuallyDrop;
    use core::ops::{Deref, DerefMut};
    use super::Consume;

    /// A zero-overhead wrapper around `T`. When a [`ConsumeOnDrop<T>`] is dropped,
    /// the underlying `T` is [`Consume::consume`]d.
    #[repr(transparent)]
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ConsumeOnDrop<T: Consume> {
        inner: ManuallyDrop<T>,
    }

    impl<T: Consume> ConsumeOnDrop<T> {
        /// Wraps a `T` in a [`ConsumeOnDrop`].
        #[inline]
        pub const fn new(value: T) -> Self {
            Self { inner: ManuallyDrop::new(value) }
        }

        /// Unwraps the underlying `T`.
        #[inline]
        pub fn into_inner(slot: Self) -> T {
            let mut slot = ManuallyDrop::new(slot);
            unsafe {
                // SAFETY: we never use slot after this function is called, since
                // we take it by value and Self is not Copy. We also don't use slot
                // again in this function, since we moved it in a ManuallyDrop to prevent
                // accidentally dropping it.
                ManuallyDrop::take(&mut slot.inner)
            }
        }
    }

    impl<T: Consume> Deref for ConsumeOnDrop<T> {
        type Target = T;

        #[inline]
        fn deref(&self) -> &Self::Target {
            self.inner.deref()
        }
    }

    impl<T: Consume> DerefMut for ConsumeOnDrop<T> {
        #[inline]
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.inner.deref_mut()
        }
    }

    impl<T: Consume> Drop for ConsumeOnDrop<T> {
        #[inline]
        fn drop(&mut self) {
            unsafe {
                // SAFETY: It is impossible to use self.inner again after Drop is called.
                ManuallyDrop::take(&mut self.inner).consume()
            }
        }
    }
}

// Note: this module doesn't use the "unsafe" keyword. It's purely
// a safe abstraction on top of the `consume_on_drop` module.
mod with_consumer {
    use core::ops::{Deref, DerefMut};
    use crate::Consume;
    use super::ConsumeOnDrop;

    /// A type implementing [`Consumer<T>`] is one which can consume a value
    /// of type `T`. In particular, any `FnOnce(T)` is also a [`Consumer<T>`].
    pub trait Consumer<T> {
        fn consume(self, other: T);
    }

    impl<T, Q: FnOnce(T)> Consumer<T> for Q {
        #[inline]
        fn consume(self, other: T) {
            self(other)
        }
    }

    #[derive(Default, Debug, Clone)]
    struct RawWithConsumer<T, Q>(T, Q);

    impl<T, Q: Consumer<T>> Consume for RawWithConsumer<T, Q> {
        #[inline]
        fn consume(self) {
            self.1.consume(self.0)
        }
    }

    /// A pair consisting of a `T` and a [`Consumer<T>`]. When this pair is
    /// dropped, the `T` will be consumed by the [`Consumer`].
    ///
    /// Note: this type does not derive traits like [`Eq`] and [`Hash`] because
    /// it may depend on context whether these traits should use only the `T`, or
    /// both the `T` and the `Q`.
    #[derive(Default, Debug, Clone)]
    pub struct WithConsumer<T, Q: Consumer<T>> {
        inner: ConsumeOnDrop<RawWithConsumer<T, Q>>,
    }

    impl<T, Q: Consumer<T>> WithConsumer<T, Q> {
        /// Builds a [`WithConsumer`] from a value and a consumer.
        #[inline]
        pub const fn new(val: T, cons: Q) -> Self {
            Self {
                inner: ConsumeOnDrop::new(
                    RawWithConsumer(val, cons))
            }
        }

        /// Extracts the underlying `T` and [`Consumer<T>`].
        #[inline]
        pub fn into_pair(x: Self) -> (T, Q) {
            let raw = ConsumeOnDrop::into_inner(x.inner);
            (raw.0, raw.1)
        }

        /// Extracts the underlying `T`, dropping the [`Consumer`]
        #[inline]
        pub fn into_inner(x: Self) -> T {
            Self::into_pair(x).0
        }

        /// Provides references to both the `T` and the [`Consumer<T>`]
        /// wrapped by `x`.
        #[inline]
        pub fn as_refs(x: &Self) -> (&T, &Q) {
            let raw = x.inner.deref();
            (&raw.0, &raw.1)
        }

        /// Provides mutable references to both the `T` and the [`Consumer<T>`]
        /// wrapped by `x`.
        #[inline]
        pub fn as_muts(x: &mut Self) -> (&mut T, &mut Q) {
            let raw = x.inner.deref_mut();
            (&mut raw.0, &mut raw.1)
        }
    }

    impl<T, Q: Consumer<T>> Deref for WithConsumer<T, Q> {
        type Target = T;

        #[inline]
        fn deref(&self) -> &Self::Target {
            Self::as_refs(self).0
        }
    }

    impl<T, Q: Consumer<T>> DerefMut for WithConsumer<T, Q> {
        #[inline]
        fn deref_mut(&mut self) -> &mut Self::Target {
            Self::as_muts(self).0
        }
    }
}