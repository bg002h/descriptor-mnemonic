//! The lowering proper. STUB until its tests exist (Task 2 replaces this file).

use super::{ComposeError, Composed, PathList, SlotOrigin};

pub(super) fn lower(
    list: &PathList,
    declared: &[Option<SlotOrigin>],
) -> Result<Composed, ComposeError> {
    let _ = (list, declared);
    unimplemented!("the wsh lowering lands with its tests")
}
