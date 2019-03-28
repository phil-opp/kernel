use core::ops::{Deref, DerefMut};
use spin::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use context::Context;
use self::sum::EccContext;

mod reed_solomon;
mod sum;

/* see https://github.com/rust-lang/rust/issues/56512
pub struct Ecc<T> where T: Sized {
    data: [Buffer; (mem::size_of::<T>() + MAX_BLOCK_SIZE - 1) / MAX_BLOCK_SIZE],
    phantom: PhantomData<T>,
}
*/

#[derive(Debug)]
pub struct EccContextRwLock(RwLock<EccContext>);

impl EccContextRwLock {
    pub fn new(context: Context) -> Self {
        EccContextRwLock(RwLock::new(EccContext::new(context)))
    }

    pub fn read(&self) -> EccContextRwLockReadGuard {
        return self.read_repair();
        let ecc_context = self.0.read();
        assert!(!ecc_context.is_corrupted(), "Context is corrupted");
        EccContextRwLockReadGuard(ecc_context)
    }

    pub fn read_repair(&self) -> EccContextRwLockReadGuard {
        let corrupted = {
            let ecc_context = self.0.read();
            ecc_context.is_corrupted()
        };
        if corrupted {
            self.0.write().repair();
        }
        EccContextRwLockReadGuard(self.0.read())
    }

    pub fn write(&self) -> EccContextRwLockWriteGuard {
        let mut ecc_context = self.0.write();
        ecc_context.recalculate_ecc();
        EccContextRwLockWriteGuard(ecc_context)
    }
}

pub struct EccContextRwLockReadGuard<'a>(RwLockReadGuard<'a, EccContext>);

impl<'a> Deref for EccContextRwLockReadGuard<'a> {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        self.0.context()
    }
}

pub struct EccContextRwLockWriteGuard<'a>(RwLockWriteGuard<'a, EccContext>);

impl<'a> Deref for EccContextRwLockWriteGuard<'a> {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        self.0.context()
    }
}

impl<'a> DerefMut for EccContextRwLockWriteGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.context_mut()
    }
}

impl<'a> Drop for EccContextRwLockWriteGuard<'a> {
    fn drop(&mut self) {
        self.0.recalculate_ecc();
        assert!(!self.0.is_corrupted());
    }
}

