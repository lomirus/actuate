use std::{
    any::{Any, TypeId},
    cell::UnsafeCell,
    collections::HashMap,
    marker::PhantomData,
    mem,
};

#[derive(Default)]
pub struct World {
    states: HashMap<TypeId, Box<dyn Any>>,
}

impl World {
    pub fn query<'a, Q: Query<'a>>(&'a mut self) -> Q {
        Q::query(&UnsafeCell::new(self))
    }
}

pub trait Query<'a> {
    fn query(world: &UnsafeCell<&'a mut World>) -> Self;
}

impl<'a, T: 'static> Query<'a> for &'a T {
    fn query(world: &UnsafeCell<&'a mut World>) -> Self {
        let world = unsafe { &mut *world.get() };
        world
            .states
            .get(&TypeId::of::<T>())
            .unwrap()
            .downcast_ref()
            .unwrap()
    }
}

pub trait System<'a>: 'static {
    type Query: Query<'a>;

    fn run(&self, query: Self::Query);
}

pub struct FnSystem<F, Marker> {
    f: F,
    _marker: PhantomData<Marker>,
}

impl<'a, F, Q> System<'a> for FnSystem<F, Q>
where
    F: Fn(Q) + 'static,
    Q: Query<'a> + 'static,
{
    type Query = Q;

    fn run(&self, query: Self::Query) {
        (self.f)(query)
    }
}

pub trait IntoSystem<'a, Marker> {
    type System: System<'a>;

    fn into_system(self) -> Self::System;
}

impl<'a, F, Q> IntoSystem<'a, Q> for F
where
    F: Fn(Q) + 'static,
    Q: Query<'a> + 'static,
{
    type System = FnSystem<F, Q>;

    fn into_system(self) -> Self::System {
        FnSystem {
            f: self,
            _marker: PhantomData,
        }
    }
}

trait AnySystem {
    unsafe fn run_any(&self, world: &UnsafeCell<&mut World>);
}

impl<'a, S: System<'a>> AnySystem for S {
    unsafe fn run_any(&self, world: &UnsafeCell<&mut World>) {
        let world = unsafe { mem::transmute(world) };
        let query = S::Query::query(world);
        self.run(query)
    }
}

#[derive(Default)]
pub struct Builder {
    systems: HashMap<TypeId, Box<dyn AnySystem>>,
}

impl Builder {
    pub fn add_system<'a, Marker>(&mut self, system: impl IntoSystem<'a, Marker>)
    where
        Self: 'a,
    {
        let s = system.into_system();
        self.systems.insert(s.type_id(), Box::new(s));
    }
}
