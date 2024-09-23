use crate::sched::rros_rq;
use kernel::ktime::*;

pub struct RrosAccount {
    #[cfg(CONFIG_RROS_RUNSTATS)]
    start: KtimeT,
    #[cfg(CONFIG_RROS_RUNSTATS)]
    total: KtimeT,
}

impl RrosAccount {
    #[cfg(CONFIG_RROS_RUNSTATS)]
    pub fn new(start: KtimeT, total: KtimeT) -> Self {
        RrosAccount { start, total }
    }

    #[cfg(CONFIG_RROS_RUNSTATS)]
    pub fn get_account_total(&self) -> KtimeT {
        self.total
    }

    #[cfg(CONFIG_RROS_RUNSTATS)]
    pub fn reset_account(&mut self) {
        self.total = 0;
        self.start = rros_get_timestamp();
    }

    #[cfg(CONFIG_RROS_RUNSTATS)]
    pub fn set_account_total(&mut self, total: KtimeT) {
        self.total = total;
    }

    #[cfg(CONFIG_RROS_RUNSTATS)]
    pub fn set_account_start(&mut self, start: KtimeT) {
        self.start = start;
    }

    #[cfg(not(CONFIG_RROS_RUNSTATS))]
    pub fn new() -> Self {
        RrosAccount {}
    }

    #[cfg(not(CONFIG_RROS_RUNSTATS))]
    pub fn get_account_total(&self) -> KtimeT {
        0
    }

    #[cfg(not(CONFIG_RROS_RUNSTATS))]
    #[allow(dead_code)]
    pub fn reset_account(&mut self) {}
}

#[cfg(CONFIG_RROS_RUNSTATS)]
fn rros_get_timestamp() -> KtimeT {
    unsafe { rros_read_clock(&RROS_MONO_CLOCK) }
}

#[cfg(CONFIG_RROS_RUNSTATS)]
// TODO:
pub fn rros_update_account(rq: Option<*mut rros_rq>) {
    match rq {
        None => return,
        Some(x) => {
            let now = rros_get_timestamp();
            unsafe {
                let total = (*x).current_account.get_account_total();
                (*x).current_account
                    .set_account_total(total + now - (*x).last_account_switch);
                (*x).last_account_switch = now;
                // smp_wmb();Not implemented
            }
        }
    }
}

#[cfg(CONFIG_RROS_RUNSTATS)]
pub fn rros_get_last_account_switch(rq: Option<*mut rros_rq>) -> KtimeT {
    match rq {
        None => return 0,
        Some(x) => unsafe {
            return (*x).last_account_switch;
        },
    }
}

#[cfg(CONFIG_RROS_RUNSTATS)]
pub fn rros_set_current_account(
    rq: Option<*mut rros_rq>,
    new_account: *mut RrosAccount,
) -> *mut RrosAccount {
    match rq {
        None => return 0 as *mut RrosAccount,
        Some(x) => unsafe {
            let prev = (*x).current_account;
            (*x).current_account = new_account;
            return prev;
        },
    }
}

#[cfg(CONFIG_RROS_RUNSTATS)]
pub fn rros_close_account(rq: Option<*mut rros_rq>, new_account: *mut RrosAccount) {
    match rq {
        None => return,
        Some(x) => unsafe {
            (*x).last_account_switch = rros_get_timestamp();
            (*x).current_account = new_account;
        },
    }
}

#[cfg(not(CONFIG_RROS_RUNSTATS))]
#[allow(dead_code)]
fn rros_get_timestamp() -> KtimeT {
    0
}

#[cfg(not(CONFIG_RROS_RUNSTATS))]
#[allow(dead_code)]
pub fn rros_update_account(_rq: Option<*mut rros_rq>) {}

#[cfg(not(CONFIG_RROS_RUNSTATS))]
#[allow(dead_code)]
pub fn rros_set_current_account(_rq: Option<*mut rros_rq>, _new_account: *mut RrosAccount) {}

#[cfg(not(CONFIG_RROS_RUNSTATS))]
#[allow(dead_code)]
pub fn rros_close_account(_rq: Option<*mut rros_rq>, _new_account: *mut RrosAccount) {}

#[cfg(not(CONFIG_RROS_RUNSTATS))]
#[allow(dead_code)]
pub fn rros_get_last_account_switch(_rq: Option<*mut rros_rq>) -> KtimeT {
    0
}

pub struct RrosCounter {
    #[cfg(CONFIG_RROS_RUNSTATS)]
    counter: u32,
}

impl RrosCounter {
    #[cfg(CONFIG_RROS_RUNSTATS)]
    pub fn new(counter: u32) -> Self {
        RrosCounter { counter }
    }

    #[cfg(CONFIG_RROS_RUNSTATS)]
    pub fn inc_counter(&mut self) {
        self.counter = self.counter + 1;
    }

    #[cfg(CONFIG_RROS_RUNSTATS)]
    pub fn get_counter(&self) -> u32 {
        self.counter
    }

    #[cfg(CONFIG_RROS_RUNSTATS)]
    pub fn set_counter(&mut self, value: u32) {
        self.counter = value;
    }

    #[cfg(not(CONFIG_RROS_RUNSTATS))]
    pub fn new() -> Self {
        RrosCounter {}
    }

    #[cfg(not(CONFIG_RROS_RUNSTATS))]
    pub fn inc_counter(&mut self) {}

    #[cfg(not(CONFIG_RROS_RUNSTATS))]
    pub fn get_counter(&self) -> u32 {
        0
    }

    #[cfg(not(CONFIG_RROS_RUNSTATS))]
    #[allow(dead_code)]
    pub fn set_counter(&mut self, _value: u32) {}
}

#[allow(dead_code)]
pub fn rros_switch_account(rq: Option<*mut rros_rq>, new_account: *mut RrosAccount) {
    rros_update_account(rq);
    rros_set_current_account(rq, new_account);
}
