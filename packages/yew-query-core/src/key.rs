use std::{
    any::TypeId,
    fmt::{self, Debug, Display, Formatter},
    ops::Deref,
    rc::Rc,
};

use self::x::TypeNameCache;

#[cfg(debug_assertions)]
thread_local! {
    static TYPE_NAMES: TypeNameCache = TypeNameCache::new();
}

/// An string key to identify a query.
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Key {
    key: Rc<str>,
}

impl Display for Key {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        std::fmt::Display::fmt(&self.key, f)
    }
}

impl Deref for Key {
    type Target = str;

    fn deref(&self) -> &str {
        self.key.as_ref()
    }
}

impl From<Rc<str>> for Key {
    fn from(key: Rc<str>) -> Self {
        Self { key }
    }
}

impl From<&'_ str> for Key {
    fn from(key: &'_ str) -> Self {
        let key: Rc<str> = Rc::from(key);
        Self::from(key)
    }
}

macro_rules! key_impl_from_to_string {
    ($type:ty) => {
        impl From<$type> for Key {
            fn from(key: $type) -> Self {
                Self::from(key.to_string().as_str())
            }
        }
    };
}

key_impl_from_to_string!(String);
key_impl_from_to_string!(char);
key_impl_from_to_string!(u8);
key_impl_from_to_string!(u16);
key_impl_from_to_string!(u32);
key_impl_from_to_string!(u64);
key_impl_from_to_string!(u128);
key_impl_from_to_string!(usize);
key_impl_from_to_string!(i8);
key_impl_from_to_string!(i16);
key_impl_from_to_string!(i32);
key_impl_from_to_string!(i64);
key_impl_from_to_string!(i128);
key_impl_from_to_string!(isize);

/// A key to identify a query.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QueryKey {
    key: Key,
    ty: TypeId,
}

impl QueryKey {
    /// Constructs a `QueryKey` for the given type and key.
    pub fn of<T: 'static>(key: Key) -> Self {
        #[cfg(debug_assertions)]
        {
            TYPE_NAMES.with(|x| x.register::<T>());
        }

        QueryKey {
            key,
            ty: TypeId::of::<T>(),
        }
    }

    /// Returns `true` if the key is for the given type.
    pub fn is_type<T: 'static>(&self) -> bool {
        TypeId::of::<T>() == self.ty
    }

    /// Returns the key of this query key.
    pub fn key(&self) -> &Key {
        &self.key
    }

    /// Returns the type of this type.
    pub fn type_id(&self) -> TypeId {
        self.ty
    }
}

impl Display for QueryKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.key)
    }
}

impl Debug for QueryKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct("QueryKey");

        debug_struct.field("key", &self.key);

        if cfg!(debug_assertions) {
            let type_name = TYPE_NAMES.with(|x| x.get(&self.ty));
            debug_struct.field("ty", &type_name);
        } else {
            debug_struct.field("ty", &self.ty);
        }

        debug_struct.finish()
    }
}

#[cfg(debug_assertions)]
mod x {
    use std::{
        any::{type_name, TypeId},
        cell::RefCell,
        collections::HashMap,
    };

    #[derive(Default)]
    pub struct TypeNameCache {
        data: RefCell<HashMap<TypeId, &'static str>>,
    }

    impl TypeNameCache {
        pub fn new() -> Self {
            Default::default()
        }

        pub fn register<T: 'static>(&self) {
            self.data
                .borrow_mut()
                .insert(TypeId::of::<T>(), type_name::<T>());
        }

        pub fn get(&self, type_id: &TypeId) -> &'static str {
            self.data
                .borrow()
                .get(type_id)
                .expect("type was not registered")
        }
    }
}
