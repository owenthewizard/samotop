use core::panic;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Debug,
    iter::{empty, once},
};

#[derive(Default)]
pub struct Store {
    /// Implementation-specific value store
    store: HashMap<TypeId, Vec<Box<dyn Any + Send + Sync>>>,
}

impl Store {
    pub fn get_or_compose<T>(&self) -> T::Target
    where
        T: ComposableComponent + 'static,
        T::Target: Clone,
    {
        T::get_or_compose(self.get_all_internal::<T>())
    }
    pub fn get_mut<T>(&mut self) -> Option<&mut T::Target>
    where
        T: SingleComponent,
        T: 'static,
    {
        self.store
            .get_mut(&TypeId::of::<T>())
            .and_then(|set| match set.len() {
                0 => None,
                1 => Some(set[0].downcast_mut().expect("downcast must succeed")),
                _ => Some(T::get_mut(
                    set.iter_mut()
                        .map(|i| i.downcast_mut::<T::Target>().into_iter())
                        .flatten(),
                )),
            })
    }
    pub fn get_ref<T>(&self) -> Option<&T::Target>
    where
        T: SingleComponent,
        T: 'static,
    {
        T::get_ref(self.get_all_internal::<T>())
    }
    pub fn get_or_insert<T, F>(&mut self, insert: F) -> &mut T::Target
    where
        T: SingleComponent + 'static,
        F: FnOnce() -> T::Target,
        T::Target: Send + Sync,
    {
        let set = self
            .store
            .entry(TypeId::of::<T>())
            .or_insert_with(|| vec![Box::new(insert())]);
        if set.len() == 1 {
            set[0].downcast_mut().expect("downcast must succeed")
        } else {
            T::get_mut(
                set.iter_mut()
                    .map(|i| i.downcast_mut::<T::Target>().into_iter())
                    .flatten(),
            )
        }
    }
    /// Set a new instance, discarding any previous - even multiple
    pub fn set<T>(&mut self, value: T::Target) -> &mut T::Target
    where
        T: Component + 'static,
        T::Target: Send + Sync,
    {
        let id = TypeId::of::<T>();
        self.store.insert(id, vec![Box::new(value)]);
        self.get_all_internal_mut::<T>()
            .next()
            .expect("must be set after set")
    }
    /// Add a new instance
    pub fn add<T>(&mut self, value: T::Target)
    where
        T: MultiComponent + 'static,
        T::Target: Send + Sync,
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
    pub fn get_all<T>(&self) -> impl Iterator<Item = &T::Target>
    where
        T: MultiComponent + 'static,
        T::Target:,
    {
        self.get_all_internal::<T>()
    }
    pub fn get_all_mut<T>(&mut self) -> impl Iterator<Item = &mut T::Target>
    where
        T: MultiComponent + 'static,
        T::Target:,
    {
        self.get_all_internal_mut::<T>()
    }
    fn get_all_internal<T>(&self) -> impl Iterator<Item = &T::Target>
    where
        T: Component + 'static,
        T::Target:,
    {
        self.store
            .get(&TypeId::of::<T>())
            .map(|v| {
                v.iter()
                    .map(|i| i.downcast_ref::<T::Target>().into_iter())
                    .flatten()
            })
            .into_iter()
            .flatten()
    }
    fn get_all_internal_mut<T>(&mut self) -> impl Iterator<Item = &mut T::Target>
    where
        T: Component + 'static,
        T::Target:,
    {
        self.store
            .get_mut(&TypeId::of::<T>())
            .map(|v| {
                v.iter_mut()
                    .map(|i| i.downcast_mut::<T::Target>().into_iter())
                    .flatten()
            })
            .into_iter()
            .flatten()
    }
}

pub trait Component {
    type Target;
}
pub trait MultiComponent: Component {
    /// Should new values be prepended instead of appended?
    /// Default behavior:
    ///     false => append
    fn prepend() -> bool {
        false
    }
}
pub trait ComposableComponent: MultiComponent {
    /// Get a single instance by value
    /// This allows us to create a new value or clone existing
    /// Default behavior:
    ///     Get a single instance if present.
    ///     Call `Self::compose(options)`  if more than one instance exists.
    ///     This is probably good enough, but the default impl of compose will panic.
    fn get_or_compose<'a, I>(mut options: I) -> Self::Target
    where
        I: Iterator<Item = &'a Self::Target> + 'a,
        Self::Target: Clone + Sized + 'a,
    {
        if let Some(first) = options.next() {
            if let Some(second) = options.next() {
                Self::compose(once(first).chain(once(second)).chain(options))
            } else {
                first.clone()
            }
        } else {
            Self::compose(empty())
        }
    }
    /// Compose a single instance from multiple.
    fn compose<'a, I>(options: I) -> Self::Target
    where
        I: Iterator<Item = &'a Self::Target> + 'a,
        Self::Target: Clone + 'a;
}
pub trait SingleComponent: Component {
    /// Get a single instance by reference.
    /// Default behavior:
    ///     Get a single instance if present.
    ///     Panic if more than one instance exists.
    /// Override this if you have a better solution.
    fn get_ref<'a, I>(mut options: I) -> Option<&'a Self::Target>
    where
        I: Iterator<Item = &'a Self::Target> + 'a,
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
    fn get_mut<'a, I>(mut options: I) -> &'a mut Self::Target
    where
        I: Iterator<Item = &'a mut Self::Target> + 'a,
    {
        if let Some(single) = options.next() {
            if options.next().is_none() {
                // exactly one item
                single
            } else {
                panic!("More than one item in store!")
            }
        } else {
            panic!("No item in store!")
        }
    }
}
impl Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(self.store.iter().map(|(k, v)| (k, format!("Any({:?})", v))))
            .finish()
    }
}
#[cfg(test)]
mod store_tests {
    use super::*;
    use crate::common::FallBack;
    use crate::io::HandlerService;
    use crate::mail::SessionLogger;
    use regex::Regex;
    use std::sync::Arc;

    #[test]
    pub fn same_service() {
        let mut sut = Store::default();
        let svc = Arc::new(SessionLogger);
        let dump0 = format!("{:#?}", svc);
        sut.set::<HandlerService>(Arc::new(FallBack));
        sut.set::<HandlerService>(svc);

        let dump1 = format!("{:#?}", sut.get_or_compose::<HandlerService>());
        assert_eq!(dump1, dump0);

        insta::assert_display_snapshot!(dump0, @"SessionLogger");
    }

    #[test]
    pub fn set_one_service() {
        let mut sut = Store::default();
        sut.set::<HandlerService>(Arc::new(FallBack));
        sut.set::<HandlerService>(Arc::new(SessionLogger));

        let dump = format!("{:#?}", sut);
        let dump = Regex::new("[0-9]+")
            .expect("regex")
            .replace_all(dump.as_str(), "--redacted--");

        insta::assert_display_snapshot!(dump, @r###"
        {
            TypeId {
                t: --redacted--,
            }: "Any([Any { .. }])",
        }
        "###);
    }
}
