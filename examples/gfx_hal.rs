use crate::gfx_simulation::Resource;
use consume_on_drop::{Consume, ConsumeOnDrop, WithConsumer};
use std::mem::{size_of, size_of_val};
use std::ops::{Deref, DerefMut};

/// In `gfx-hal`, resources must be consumed by custom functions which take `self` by value. It would
/// be quite a bit more convenient to be able to use [`drop`] normally. We can solve this using
/// [`ConsumeOnDrop`]. Here's a simplified version. This is inspired by [this question](https://stackoverflow.com/questions/53778961/is-it-possible-to-drop-and-consume-self-at-the-end-of-scope-at-the-same-time)
/// at StackOverflow.

/// Here is some dumbed-down code for resource creation and destruction. We take everything in
/// this module as given to us by a library.
mod gfx_simulation {
    pub struct Resource(());

    impl Resource {
        pub fn create_resource() -> Resource {
            Resource(())
        }

        pub fn borrow_resource(&self) {
            println!("We did something with the borrowed resource!")
        }

        pub fn borrow_mut_resource(&mut self) {
            println!("We did something with the mutably borrowed resource!")
        }

        pub fn destroy_resource(self) {
            println!("We destroyed the resource.");
        }
    }
}

/// We can't implement [`Consume`] on the library type [`Resource`], so we need a wrapper.
#[repr(transparent)]
pub struct ConsumableResource(Resource);

impl ConsumableResource {
    pub fn into_inner(self) -> Resource {
        self.0
    }
}

impl Default for ConsumableResource {
    fn default() -> Self {
        Self(Resource::create_resource())
    }
}

impl Deref for ConsumableResource {
    type Target = Resource;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ConsumableResource {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Consume for ConsumableResource {
    fn consume(self) {
        self.into_inner().destroy_resource()
    }
}

type WrappedResource = ConsumeOnDrop<ConsumableResource>;

fn main() {
    let mut wrapped_resource = WrappedResource::default();

    // wrapped_resource takes up exactly as much space as a Resource. In fact, they are guaranteed
    // to have exactly the same runtime representation due to #[repr(transparent)].
    assert_eq!(size_of::<WrappedResource>(), size_of::<Resource>());
    wrapped_resource.borrow_resource();
    wrapped_resource.borrow_mut_resource();
    drop(wrapped_resource);

    println!("Finished with first wrapped resource.");

    // Alternately, we may not need our resources to have a namable type. If that's true, we can use
    // WithConsumer instead. Unfortunately, we can't name the type of wrapped_resource, so we can't
    // use it in monomorphic functions. The benefit is that we get to skip the boilerplate
    // of writing ConsumableResource.

    let mut wrapped_resource =
        WithConsumer::new(Resource::create_resource(), Resource::destroy_resource);
    // In this case, wrapped_resource takes up exactly as much space as a resource. This is a zero-cost
    // abstraction.
    assert_eq!(size_of_val(&wrapped_resource), size_of::<Resource>());

    wrapped_resource.borrow_resource();
    wrapped_resource.borrow_mut_resource();
    // We implicitly drop wrapped_resource here when we reassign the variable to a new value.
    wrapped_resource = WithConsumer::new(Resource::create_resource(), Resource::destroy_resource);
    println!("Finished with second resource. We won't destroy the last one.");
    let resource = WithConsumer::into_inner(wrapped_resource);
    drop(resource); // this does nothing
}
