use core::{
    mem,
    ops::{Deref, DerefMut},
};
use reed_solomon::{Encoder, Decoder, DecoderError};
use spin::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use context::Context;

const ECC_LEN: usize = 8;
const MAX_BLOCK_SIZE: usize = 50 - ECC_LEN; //255 - ECC_LEN;

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
        Self(RwLock::new(EccContext::new(context)))
    }

    pub fn read(&self) -> EccContextRwLockReadGuard {
        return self.read_repair();
        let ecc_context = self.0.read();
        //assert!(!ecc_context.is_corrupted(), "Context is corrupted");
        if ecc_context.is_corrupted() {
            println!("Context is corrupted on read");
            println!("stored ECC:     {:x?}", ecc_context.ecc);
            EccContext::calculate_ecc(&ecc_context.context);
        }
        EccContextRwLockReadGuard(ecc_context)
    }

    pub fn read_repair(&self) -> EccContextRwLockReadGuard {
        {
            let ecc_context = self.0.read();
            if ecc_context.is_corrupted() {
                println!("Context is corrupted on read");
                println!("stored ECC:     {:x?}", ecc_context.ecc);
                println!("calculated ECC: {:x?}", EccContext::calculate_ecc(&ecc_context.context));
            }
        }
        {
            self.0.write().repair();
        }
        EccContextRwLockReadGuard(self.0.read())
    }

    pub fn write(&self) -> EccContextRwLockWriteGuard {
        let mut ecc_context = self.0.write();
        ecc_context.repair();
        EccContextRwLockWriteGuard(ecc_context)
    }
}

pub struct EccContextRwLockReadGuard<'a>(RwLockReadGuard<'a, EccContext>);

impl<'a> Deref for EccContextRwLockReadGuard<'a> {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.0.context
    }
}

pub struct EccContextRwLockWriteGuard<'a>(RwLockWriteGuard<'a, EccContext>);

impl<'a> Deref for EccContextRwLockWriteGuard<'a> {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.0.context
    }
}

impl<'a> DerefMut for EccContextRwLockWriteGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.context
    }
}

impl<'a> Drop for EccContextRwLockWriteGuard<'a> {
    fn drop(&mut self) {
        self.0.ecc = EccContext::calculate_ecc(&self.0.context);
        assert!(!self.0.is_corrupted());
    }
}

#[derive(Debug)]
pub struct EccContext {
    context: Context,
    ecc: [[u8; ECC_LEN]; (mem::size_of::<Context>() + MAX_BLOCK_SIZE - 1) / MAX_BLOCK_SIZE],
}

impl EccContext {
    pub fn new(context: Context) -> Self {
        let ecc = Self::calculate_ecc(&context);
        let ret = Self {context, ecc};
        assert!(!ret.is_corrupted());
        ret
    }

    fn calculate_ecc(context: &Context) -> [[u8; ECC_LEN]; (mem::size_of::<Context>() + MAX_BLOCK_SIZE - 1) / MAX_BLOCK_SIZE] {
        let data_ptr = context as *const Context as *const [u8; mem::size_of::<Context>()];
        let data = unsafe { &(*data_ptr)[..] };
        let encoder = Encoder::new(ECC_LEN);
        
        let mut ecc = [[0; ECC_LEN]; (mem::size_of::<Context>() + MAX_BLOCK_SIZE - 1) / MAX_BLOCK_SIZE];

        for (i, chunk) in data.chunks(MAX_BLOCK_SIZE).enumerate() {
            let buffer = encoder.encode(chunk);
            ecc[i].copy_from_slice(buffer.ecc());
        }

        ecc
    }

    fn repair(&mut self) {
        let Self {context, ecc} = self;

        let data_ptr = context as *mut Context as *mut [u8; mem::size_of::<Context>()];
        let data = unsafe { &mut (*data_ptr)[..] };
        let decoder = Decoder::new(ECC_LEN);

        for (i, chunk) in data.chunks_mut(MAX_BLOCK_SIZE).enumerate() {
            let mut array = [0; MAX_BLOCK_SIZE + ECC_LEN];
            let data = &mut array[..chunk.len() + ECC_LEN];
            data[..chunk.len()].copy_from_slice(chunk);
            data[chunk.len()..].copy_from_slice(&ecc[i]);
            let (buffer, error_count) = decoder.correct_err_count(data, None).unwrap_or_else(|err| match err {
                DecoderError::TooManyErrors => panic!("Context is unrecoverably corrupted"),
            });

            if error_count != 0 {
                println!("Correcting {} errors in context", error_count);
                chunk.copy_from_slice(buffer.data());
                ecc[i].copy_from_slice(buffer.ecc());
            }
        }
    }

    fn is_corrupted(&self) -> bool {
        let Self {context, ecc} = self;

        let data_ptr = context as *const Context as *const [u8; mem::size_of::<Context>()];
        let data = unsafe { &(*data_ptr)[..] };
        let decoder = Decoder::new(ECC_LEN);

        for (i, chunk) in data.chunks(MAX_BLOCK_SIZE).enumerate() {
            let mut array = [0; MAX_BLOCK_SIZE + ECC_LEN];
            let data = &mut array[..chunk.len() + ECC_LEN];
            data[..chunk.len()].copy_from_slice(chunk);
            data[chunk.len()..].copy_from_slice(&ecc[i]);
            if decoder.is_corrupted(data) {
                println!("chunk {}/{} is corrupted: {:x?} {:x?}", i, (mem::size_of::<Context>() + MAX_BLOCK_SIZE - 1) / MAX_BLOCK_SIZE -1 , chunk, ecc[i]);
                return true
            }
        }
        false
    }
}
