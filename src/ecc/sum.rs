use core::mem;
use spin::RwLock;

use context::Context;

#[derive(Debug)]
pub struct EccContext {
    context: Context,
    sum: u64,
}

impl EccContext {
    pub fn new(context: Context) -> Self {
        let sum = Self::calculate_sum(&context);
        let ret = Self {context, sum};
        assert!(!ret.is_corrupted());
        ret
    }

    pub fn context(&self) -> &Context {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    fn calculate_sum(context: &Context) -> u64 {
        let data_ptr = context as *const Context as *const [u64; mem::size_of::<Context>() / 8];
        let data = unsafe { &(*data_ptr)[..] };

        let mut sum: u64 = 0;
        for &word in data {
            sum.wrapping_add(word);
        }
        sum
    }

    pub fn recalculate_ecc(&mut self) {
        self.sum = Self::calculate_sum(&self.context);
    }

    pub fn repair(&mut self) {
        unimplemented!(); // TODO store copy of context
    }

    pub fn is_corrupted(&self) -> bool {
        let Self {context, sum} = self;

        let correct_sum = Self::calculate_sum(&context);

        if *sum != correct_sum {
            println!("context is corrupted");
            true
        } else {
            false
        }
    }
}
