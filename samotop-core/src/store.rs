use core::panic;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    iter::once,
};

#[derive(Debug, Default)]
pub struct Store {
    /// Implementation-specific value store
    store: HashMap<TypeId, Vec<Box<dyn Any + Send + Sync>>>,
}

impl Store {
    pub fn get_or_compose<T>(&self) -> Option<T>
    where
        T: ComposableComponent,
        T: 'static,
    {
        T::get_or_compose(self.get_all_internal())
    }
    pub fn get_mut<T>(&mut self) -> Option<&mut T>
    where
        T: SingleComponent,
        T: 'static,
    {
        T::get_mut(self.get_all_internal_mut())
    }
    pub fn get_ref<T>(&self) -> Option<&T>
    where
        T: SingleComponent,
        T: 'static,
    {
        T::get_ref(self.get_all_internal())
    }
    pub fn get_or_insert<T, F>(&mut self, insert: F) -> Option<&mut T>
    where
        T: SingleComponent,
        F: FnOnce() -> T,
        T: Send + Sync + 'static,
    {
        T::get_mut(
            self.store
                .entry(TypeId::of::<T>())
                .or_insert_with(|| vec![Box::new(insert())])
                .iter_mut()
                .map(|i| i.downcast_mut::<T>().into_iter())
                .flatten(),
        )
    }
    /// Set a new instance, discarding any previous - even multiple
    pub fn set<T>(&mut self, value: T)
    where
        T: Send + Sync + 'static,
        T: SingleComponent,
    {
        self.store.insert(TypeId::of::<T>(), vec![Box::new(value)]);
    }
    /// Add a new instance
    pub fn add<T>(&mut self, value: T)
    where
        T: Send + Sync + 'static,
        T: MultiComponent,
    {
        let set = self
            .store
            .entry(TypeId::of::<T>())
            .or_insert_with(|| vec![]);
        if T::prepend() {
            set.insert(0, Box::new(value))
        } else {
            set.push(Box::new(value))
        }
    }
    pub fn get_all<T>(&self) -> impl Iterator<Item = &T>
    where
        T: MultiComponent,
        T: 'static,
    {
        self.get_all_internal()
    }
    pub fn get_all_mut<T>(&mut self) -> impl Iterator<Item = &mut T>
    where
        T: MultiComponent,
        T: 'static,
    {
        self.get_all_internal_mut()
    }
    fn get_all_internal<T>(&self) -> impl Iterator<Item = &T>
    where
        T: 'static,
    {
        self.store
            .get(&TypeId::of::<T>())
            .map(|v| {
                v.iter()
                    .map(|i| i.downcast_ref::<T>().into_iter())
                    .flatten()
            })
            .into_iter()
            .flatten()
    }
    fn get_all_internal_mut<T>(&mut self) -> impl Iterator<Item = &mut T>
    where
        T: 'static,
    {
        self.store
            .get_mut(&TypeId::of::<T>())
            .map(|v| {
                v.iter_mut()
                    .map(|i| i.downcast_mut::<T>().into_iter())
                    .flatten()
            })
            .into_iter()
            .flatten()
    }
}

pub trait MultiComponent {
    /// Should new values be prepended instead of appended?
    /// Default behavior:
    ///     false => append
    fn prepend() -> bool {
        false
    }
}
pub trait ComposableComponent: MultiComponent + Clone {
    /// Get a single instance by value
    /// This allows us to create a new value or clone existing
    /// Default behavior:
    ///     Get a single instance if present.
    ///     Call `Self::compose(options)`  if more than one instance exists.
    ///     This is probably good enough, but the default impl of compose will panic.
    fn get_or_compose<'a, I>(mut options: I) -> Option<Self>
    where
        I: Iterator<Item = &'a Self> + 'a,
        Self: Sized + 'a,
    {
        let first = options.next()?;
        if let Some(second) = options.next() {
            Self::compose(once(first).chain(once(second)).chain(options))
        } else {
            Some(first.clone())
        }
    }
    /// Compose a single instance from multiple.
    fn compose<'a, I>(options: I) -> Option<Self>
    where
        I: Iterator<Item = &'a Self> + 'a,
        Self: 'a;
}
pub trait SingleComponent {
    /// Get a single instance by reference.
    /// Default behavior:
    ///     Get a single instance if present.
    ///     Panic if more than one instance exists.
    /// Override this if you have a better solution.
    fn get_ref<'a, I>(mut options: I) -> Option<&'a Self>
    where
        I: Iterator<Item = &'a Self> + 'a,
    {
        let first = options.next()?;
        if options.next().is_some() {
            panic!("More than one item in store!")
        } else {
            Some(first)
        }
    }
    /// Get a single instance by mutable reference.
    /// Default behavior:
    ///     Get a single instance if present.
    ///     Panic if more than one instance exists.
    /// Override this if you have a better solution.
    fn get_mut<'a, I>(mut options: I) -> Option<&'a mut Self>
    where
        I: Iterator<Item = &'a mut Self> + 'a,
    {
        let first = options.next()?;
        if options.next().is_some() {
            panic!("More than one item in store!")
        } else {
            Some(first)
        }
    }
}
