use kernel::ktime::*;
pub const RROS_INFINITE: KtimeT = 0;
pub const RROS_NONBLOCK: KtimeT = i64::MAX;

pub fn timeout_infinite(kt: KtimeT) -> bool {
    kt == 0
}

pub fn timeout_nonblock(kt: KtimeT) -> bool {
    kt < 0
}

#[allow(dead_code)]
pub fn timeout_valid(kt: KtimeT) -> bool {
    kt > 0
}

#[derive(Clone, PartialEq, Eq, Debug, Copy)]
pub enum RrosTmode {
    RrosRel,
    RrosAbs,
}
