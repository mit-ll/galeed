use std::collections::HashMap;
use core::cell::Cell;
use core::marker::PhantomData;

type IdType = i32;

#[repr(C)]
pub struct FakePtr<T> {
    pub id: IdType,
    phantom: PhantomData<T>,
}

impl <T> Clone for FakePtr<T> {
    fn clone(&self) -> FakePtr<T> {
        FakePtr {id: self.id, phantom: PhantomData}
    }
}

impl<T> Copy for FakePtr<T> {}

pub struct FakePtrMap<T> {
    pub ptr_to_t: Cell<HashMap<IdType, T>>,
    pub next_id: Cell<IdType>
}

impl<T> FakePtrMap<T>{
    pub fn new() -> FakePtrMap<T> {
        FakePtrMap{ptr_to_t: Cell::new(HashMap::new()), 
                   next_id: Cell::new(1)}
    }
    pub fn to_fake_ptr(self: &Self, t: T) -> FakePtr<T> {
        let fake_id = self.next_id.get();
        let mut inner_map = self.ptr_to_t.take();
        inner_map.insert(fake_id, t);
        self.ptr_to_t.set(inner_map);
        FakePtr::<T>{
            id: self.next_id.replace(fake_id + 1),
            phantom: PhantomData
        }
    }

    pub fn recover(self: &Self, p: FakePtr<T>) -> T {
        let mut inner_map = self.ptr_to_t.take();
        let result = inner_map.remove(&p.id).expect("Incorrect FakePtr requested, erroring out");
        self.ptr_to_t.set(inner_map);
        result
    }
}