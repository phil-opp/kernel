use core::mem;
use reed_solomon::{Encoder, Decoder, DecoderError};
use spin::RwLock;

use context::Context;

const ECC_LEN: usize = 8;
const MAX_BLOCK_SIZE: usize = 255 - ECC_LEN;

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

    pub fn context(&self) -> &Context {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.context
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

    pub fn recalculate_ecc(&mut self) {
        self.ecc = Self::calculate_ecc(&self.context);
    }

    pub fn repair(&mut self) {
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

    pub fn is_corrupted(&self) -> bool {
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
